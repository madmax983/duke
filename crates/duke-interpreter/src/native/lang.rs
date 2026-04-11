/// Native: `String.indent(int) -> String` — prepends `n` spaces to each line.
/// Negative `n` removes up to `|n|` leading spaces per line (Java 12+ semantics).
pub(crate) fn native_string_indent(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let n = extract_int_arg(args, 1)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let result: String = if n >= 0 {
        let prefix = " ".repeat(n as usize);
        s.lines()
            .map(|line| {
                let mut out = String::with_capacity(prefix.len() + line.len() + 1);
                out.push_str(&prefix);
                out.push_str(line);
                out.push('\n');
                out
            })
            .collect()
    } else {
        let remove = (-n) as usize;
        s.lines()
            .map(|line| {
                let stripped = line.trim_start_matches(' ');
                let leading = line.len() - stripped.len();
                let keep = leading.saturating_sub(remove);
                let spaces = " ".repeat(keep);
                let mut out = String::with_capacity(keep + stripped.len() + 1);
                out.push_str(&spaces);
                out.push_str(stripped);
                out.push('\n');
                out
            })
            .collect()
    };
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Double.min(double,double)double`
pub(crate) fn native_double_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Double(a.min(b))))
}

/// Native: `Double.max(double,double)double`
pub(crate) fn native_double_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Double(a.max(b))))
}

/// Native: `Double.compare(double,double)int`
pub(crate) fn native_double_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        Some(Slot::Float(v)) => f64::from(*v),
        _ => 0.0,
    };
    Ok(Some(Slot::Int(a.total_cmp(&b) as i32)))
}

/// Native: `Long.min(long,long)long`
pub(crate) fn native_long_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Long(a.min(b))))
}

/// Native: `Long.max(long,long)long`
pub(crate) fn native_long_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Long(a.max(b))))
}

/// Native: `Long.compare(long,long)int`
pub(crate) fn native_long_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        Some(Slot::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(Some(Slot::Int(a.cmp(&b) as i32)))
}

/// Native: `Integer.min(int,int)int`
pub(crate) fn native_integer_min(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.min(b))))
}

/// Native: `Integer.max(int,int)int`
pub(crate) fn native_integer_max(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.max(b))))
}

/// Native: `Integer.compare(int,int)int` — returns negative/zero/positive.
pub(crate) fn native_integer_compare(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(a.cmp(&b) as i32)))
}

/// Native: `String.chars()IntStream` — returns char code points as an `IntStream`.
pub(crate) fn native_string_chars(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
        let values: Vec<i32> = s.chars().map(|c| c as i32).collect();
    Ok(Some(Slot::Reference(Some(make_int_stream(heap, values)))))
}

/// Native: `StringJoiner.length()I` — length of the `toString()` result.
pub(crate) fn native_stringjoiner_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // Reuse toString and measure
    let result = native_stringjoiner_tostring(args, heap, out, control)?;
    let len = match result {
        Some(Slot::Reference(Some(r))) => heap.get(r)?.string_value.as_deref().unwrap_or("").len(),
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

/// Native: `StringJoiner.toString()String` — builds the joined result.
///
/// ⚡ Bolt Optimization: Eliminated intermediate `Vec<String>` allocation and format
/// macro overhead by appending directly to a single String buffer.
pub(crate) fn native_stringjoiner_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(this_ref)?.fields.clone();
    // elements start at index 4
    let elems: Vec<Slot> = fields.get(4..).map(<[Slot]>::to_vec).unwrap_or_default();
    if elems.is_empty() {
        let empty_slot = fields.get(3).copied().unwrap_or(Slot::Reference(None));
        let prefix = slot_to_string(
            fields.get(1).copied().unwrap_or(Slot::Reference(None)),
            heap,
        );
        let suffix = slot_to_string(
            fields.get(2).copied().unwrap_or(Slot::Reference(None)),
            heap,
        );
        let empty = slot_to_string(empty_slot, heap);
        // If prefix+suffix are both empty, return emptyValue; otherwise prefix+suffix
        let result = if prefix.is_empty() && suffix.is_empty() {
            empty
        } else {
            format!("{prefix}{suffix}")
        };
        let r = heap.allocate_string(result);
        return Ok(Some(Slot::Reference(Some(r))));
    }
    let delim = slot_to_string(
        fields.first().copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    let prefix = slot_to_string(
        fields.get(1).copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    let suffix = slot_to_string(
        fields.get(2).copied().unwrap_or(Slot::Reference(None)),
        heap,
    );
    // ⚡ Bolt: Eliminate intermediate Vec<String> allocation and format! macro overhead
    // by appending directly to a single String buffer.
    let mut result = String::new();
    result.push_str(&prefix);
    for (i, elem) in elems.iter().enumerate() {
        if i > 0 {
            result.push_str(&delim);
        }
        result.push_str(&slot_to_string(*elem, heap));
    }
    result.push_str(&suffix);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringJoiner.add(CharSequence)StringJoiner` — append element.
pub(crate) fn native_stringjoiner_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    heap.get_mut(this_ref)?.fields.push(elem);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringJoiner.<init>(CharSequence,CharSequence,CharSequence)V` — delim + prefix + suffix.
pub(crate) fn native_stringjoiner_init_prefix_suffix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delim_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let prefix_slot = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let suffix_slot = args.get(3).copied().unwrap_or(Slot::Reference(None));
    let empty_ref = heap.allocate_string(String::new());
    heap.get_mut(this_ref)?.fields[0] = delim_slot;
    heap.get_mut(this_ref)?.fields[1] = prefix_slot;
    heap.get_mut(this_ref)?.fields[2] = suffix_slot;
    heap.get_mut(this_ref)?.fields[3] = Slot::Reference(Some(empty_ref));
    Ok(None)
}

/// Native: `StringJoiner.<init>(CharSequence)V` — delimiter only.
pub(crate) fn native_stringjoiner_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let delim_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let empty_ref = heap.allocate_string(String::new());
    let prefix_ref = heap.allocate_string(String::new());
    let suffix_ref = heap.allocate_string(String::new());
    heap.get_mut(this_ref)?.fields[0] = delim_slot;
    heap.get_mut(this_ref)?.fields[1] = Slot::Reference(Some(prefix_ref));
    heap.get_mut(this_ref)?.fields[2] = Slot::Reference(Some(suffix_ref));
    heap.get_mut(this_ref)?.fields[3] = Slot::Reference(Some(empty_ref));
    Ok(None)
}

/// Native: `StringBuffer.length()I`.
pub(crate) fn native_stringbuffer_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let len = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .unwrap_or("")
        .len();
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

/// Native: `StringBuffer.toString()String`.
pub(crate) fn native_stringbuffer_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringBuffer.append(...)StringBuffer` — append any type; returns `this`.
pub(crate) fn native_stringbuffer_append(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let frag = match args.get(1).copied().unwrap_or(Slot::Reference(None)) {
        Slot::Reference(Some(r)) => heap
            .get(r)?
            .string_value
            .clone()
            .unwrap_or_else(|| "null".to_string()),
        Slot::Reference(None) => "null".to_string(),
        Slot::Int(n) => n.to_string(),
        Slot::Long(n) => n.to_string(),
        Slot::Double(d) => format!("{d}"),
        Slot::Float(f) => format!("{f}"),
        Slot::ReturnAddress(_) => String::new(),
    };
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    buf.push_str(&frag);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuffer.<init>(Ljava/lang/String;)V` — init with string.
pub(crate) fn native_stringbuffer_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = match args.get(1).copied() {
        Some(Slot::Reference(Some(r))) => heap.get(r)?.string_value.clone().unwrap_or_default(),
        _ => String::new(),
    };
    heap.get_mut(this_ref)?.string_value = Some(s);
    Ok(None)
}

/// Native: `StringBuffer.<init>()V` — empty buffer.
pub(crate) fn native_stringbuffer_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.string_value = Some(String::new());
    Ok(None)
}

/// Native: `String.replaceFirst(String,String)String` — regex replace first.
pub(crate) fn native_string_replace_first_regex(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let pat_ref = extract_ref_arg(args, 1)?;
    let repl_ref = extract_ref_arg(args, 2)?;
    let input = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replaceAll(String,String)String` — regex replace all.
pub(crate) fn native_string_replace_all_regex(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let pat_ref = extract_ref_arg(args, 1)?;
    let repl_ref = extract_ref_arg(args, 2)?;
    let input = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace_all(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.matches(String)Z` — full-string regex match.
pub(crate) fn native_string_matches_regex(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let pat_ref = extract_ref_arg(args, 1)?;
    let input = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let matched = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    Ok(Some(Slot::Int(i32::from(matched))))
}

/// Native: `String.lines()Stream` — splits on newlines, wraps in Stream.
pub(crate) fn native_string_lines(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let text = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.len();
    let stream_ref = heap.allocate("duke/util/Stream".to_string(), n + 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(i32::try_from(n).unwrap_or(0));
    for (i, line) in lines.iter().enumerate() {
        let s_ref = heap.allocate_string((*line).to_string());
        heap.get_mut(stream_ref)?.fields[i + 1] = Slot::Reference(Some(s_ref));
    }
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `String.codePointAt(I)I` — returns the Unicode code point at the given index.
pub(crate) fn native_string_code_point_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let idx = usize::try_from(extract_int_arg(args, 1)?).unwrap_or(0);
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let cp = s
        .chars()
        .nth(idx)
        .map_or(0_i32, |c| i32::try_from(u32::from(c)).unwrap_or(0));
    Ok(Some(Slot::Int(cp)))
}

/// Native: `String.lastIndexOf(String)I` — finds last occurrence of substring.
pub(crate) fn native_string_last_index_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let sub_ref = extract_ref_arg(args, 1)?;
    let this_str = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let sub_str = heap.get(sub_ref)?.string_value.clone().unwrap_or_default();
    let result = this_str
        .rfind(sub_str.as_str())
        .and_then(|byte_pos| i32::try_from(this_str[..byte_pos].chars().count()).ok())
        .unwrap_or(-1);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.indexOf(int)I` — finds first occurrence of char (as Unicode code point).
pub(crate) fn native_string_index_of_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let ch = char::from_u32(extract_int_arg(args, 1)? as u32).unwrap_or('\0');
    let idx = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .and_then(|s| s.char_indices().find(|(_, c)| *c == ch).map(|(i, _)| i))
        .and_then(|byte_pos| {
            heap.get(this_ref).ok().and_then(|o| {
                o.string_value
                    .as_deref()
                    .map(|s| s[..byte_pos].chars().count())
            })
        });
    let result = idx.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.join(CharSequence, CharSequence[])String` — joins array elements with delimiter.
pub(crate) fn native_string_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let delim_ref = extract_ref_arg(args, 0)?;
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // args[1] can be an Object[] array (varargs) or a single Iterable (ArrayList)
    let parts: Vec<String> = match args.get(1) {
        Some(Slot::Reference(Some(arr_ref))) => {
            let obj = heap.get(*arr_ref)?;
            if obj.class_name.starts_with('[') {
                // It's an array — fields are the elements.
                let len = obj.fields.len();
                let mut result = Vec::with_capacity(len);
                let slots: Vec<Slot> = obj.fields.clone();
                let _ = obj;
                for slot in slots {
                    let s = match slot {
                        Slot::Reference(Some(r)) => heap
                            .get(r)?
                            .string_value
                            .clone()
                            .unwrap_or_else(|| "null".to_string()),
                        Slot::Reference(None) => "null".to_string(),
                        Slot::Int(n) => n.to_string(),
                        Slot::Long(n) => n.to_string(),
                        other => format!("{other:?}"),
                    };
                    result.push(s);
                }
                result
            } else {
                // ArrayList or similar — fields[0]=size, fields[1..]=elements
                let size_val = match obj.fields.first() {
                    Some(Slot::Int(n)) => usize::try_from(*n).unwrap_or(0),
                    _ => 0,
                };
                let elems: Vec<Slot> =
                    obj.fields[1..=size_val.min(obj.fields.len().saturating_sub(1))].to_vec();
                let _ = obj;
                let mut result = Vec::with_capacity(size_val);
                for slot in elems {
                    let s = match slot {
                        Slot::Reference(Some(r)) => heap
                            .get(r)?
                            .string_value
                            .clone()
                            .unwrap_or_else(|| "null".to_string()),
                        Slot::Reference(None) => "null".to_string(),
                        Slot::Int(n) => n.to_string(),
                        Slot::Long(n) => n.to_string(),
                        other => format!("{other:?}"),
                    };
                    result.push(s);
                }
                result
            }
        }
        _ => Vec::new(),
    };
    let joined = parts.join(&delim);
    let r = heap.allocate_string(joined);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.formatted(Object[])String` — instance alias for `String.format(this, args)`.
pub(crate) fn native_string_formatted(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // args[0] = this (the format string), args[1] = Object[] varargs
    let fmt_ref = extract_ref_arg(args, 0)?;
    let arr_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    native_string_format(
        &[Slot::Reference(Some(fmt_ref)), arr_slot],
        heap,
        out,
        control,
    )
}

/// Native: `String.repeat(int)String` — repeats this string n times.
pub(crate) fn native_string_repeat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let n = usize::try_from(extract_int_arg(args, 1)?.max(0)).unwrap_or(0);
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.repeat(n));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.isBlank()Z` — true if empty or all whitespace.
pub(crate) fn native_string_is_blank(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let blank = heap.get(this_ref).map_or(true, |o| {
        o.string_value
            .as_deref()
            .is_none_or(|s| s.chars().all(char::is_whitespace))
    });
    Ok(Some(Slot::Int(i32::from(blank))))
}

/// Native: `String.stripTrailing()String` — removes trailing Unicode whitespace.
pub(crate) fn native_string_strip_trailing(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.trim_end().to_owned());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.stripLeading()String` — removes leading Unicode whitespace.
pub(crate) fn native_string_strip_leading(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.trim_start().to_owned());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.strip()String` — removes leading and trailing Unicode whitespace.
pub(crate) fn native_string_strip(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.trim().to_owned());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.intern()String` — returns canonical string (identity for our heap strings).
pub(crate) fn native_string_intern(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    // In our interpreter, string equality is by value already; intern = identity.
    Ok(Some(args.first().copied().unwrap_or(Slot::Reference(None))))
}

/// Native: `Double.compareTo(Object)` — compares two boxed Doubles.
pub(crate) fn native_double_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let double_val = |s: &Slot| -> VmResult<f64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Double(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => double_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = double_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    // Use total_cmp: implements Java's total order where NaN > +∞ > … > -∞.
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}

/// Native: `Double.isNaN(D)Z` — returns 1 if value is NaN.
pub(crate) fn native_double_isnan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Double(v)) => Ok(Some(Slot::Int(i32::from(v.is_nan())))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Character.charValue()C` — unbox Character to char.
pub(crate) fn native_char_charvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

/// Native: `Character.valueOf(C)Ljava/lang/Character;` — box a char.
pub(crate) fn native_char_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int (char)",
                got: "other",
            });
        }
    };
    let r = heap.allocate("java/lang/Character".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Character.isLetterOrDigit(C)Z`
pub(crate) fn native_char_is_letter_or_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphanumeric()))))
}

/// Native: `Character.toLowerCase(C)C`
pub(crate) fn native_char_to_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    let lower = ch.to_lowercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(lower as i32)))
}

/// Native: `Character.toUpperCase(C)C`
pub(crate) fn native_char_to_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    let upper = ch.to_uppercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(upper as i32)))
}

/// Native: `Character.isLowerCase(C)Z`
pub(crate) fn native_char_is_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_lowercase()))))
}

/// Native: `Character.isUpperCase(C)Z`
pub(crate) fn native_char_is_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_uppercase()))))
}

/// Native: `Character.isWhitespace(C)Z`
pub(crate) fn native_char_is_whitespace(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_whitespace()))))
}

/// Native: `Character.isLetter(C)Z`
pub(crate) fn native_char_is_letter(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphabetic()))))
}

/// Native: `Character.isDigit(C)Z`
pub(crate) fn native_char_is_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_ascii_digit()))))
}

/// Native: `StringBuilder.length()I`
pub(crate) fn native_sb_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let len = heap
        .get(this_ref)?
        .string_value
        .as_ref()
        .map_or(0, String::len);
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

/// Native: `StringBuilder.charAt(int)C` — returns char at given index.
pub(crate) fn native_sb_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let ch = heap
        .get(this_ref)?
        .string_value
        .as_deref()
        .and_then(|s| usize::try_from(index).ok().and_then(|i| s.chars().nth(i)))
        .unwrap_or('\0');
    Ok(Some(Slot::Int(ch as i32)))
}

/// Native: `StringBuilder.reverse()StringBuilder` — reverses the character sequence.
pub(crate) fn native_sb_reverse(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    *buf = buf.chars().rev().collect();
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.deleteCharAt(int)StringBuilder` — removes single char at index.
pub(crate) fn native_sb_delete_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    if let Some(i) = usize::try_from(index)
        .ok()
        .and_then(|i| buf.char_indices().nth(i).map(|(b, _)| b))
    {
        buf.remove(i);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.delete(int, int)StringBuilder` — removes chars in [start, end).
pub(crate) fn native_sb_delete(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let start = extract_int_arg(args, 1)?;
    let end = extract_int_arg(args, 2)?;
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let start_byte = usize::try_from(start)
        .ok()
        .and_then(|i| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    let end_byte = usize::try_from(end)
        .ok()
        .and_then(|i| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    buf.drain(start_byte..end_byte);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.insert(int, char)StringBuilder` — inserts char at index.
pub(crate) fn native_sb_insert_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let offset = extract_int_arg(args, 1)?;
    let ch = char::from_u32(extract_int_arg(args, 2)? as u32).unwrap_or('\0');
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let byte_idx = offset
        .try_into()
        .ok()
        .and_then(|i: usize| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    buf.insert(byte_idx, ch);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.insert(int, String)StringBuilder` — inserts string at index.
pub(crate) fn native_sb_insert_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let offset = extract_int_arg(args, 1)?;
    let s = match args.get(2) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        Some(Slot::Reference(None)) | None => "null".to_string(),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "String",
                got: "other",
            });
        }
    };
    let buf = heap
        .get_mut(this_ref)?
        .string_value
        .get_or_insert_with(String::new);
    let byte_idx = offset
        .try_into()
        .ok()
        .and_then(|i: usize| buf.char_indices().nth(i).map(|(b, _)| b))
        .unwrap_or(buf.len());
    buf.insert_str(byte_idx, &s);
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.toString()Ljava/lang/String;`
pub(crate) fn native_sb_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let content = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(content);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringBuilder.append(C)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('\0'),
        _ => '\0',
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push(val);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(Z)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v != 0,
        _ => false,
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(if val { "true" } else { "false" });
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(F)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_float_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(D)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_double_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(J)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_long_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(I)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = extract_int_arg(args, 1)?;
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;`
pub(crate) fn native_sb_append_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let append_str = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        _ => "null".to_string(),
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&append_str);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.<init>(Ljava/lang/String;)V` — init with string.
pub(crate) fn native_sb_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let init_str = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        _ => String::new(),
    };
    let obj = heap.get_mut(this_ref)?;
    obj.string_value = Some(init_str);
    Ok(None)
}

/// Native: `StringBuilder.<init>()V` — initialise empty buffer.
pub(crate) fn native_sb_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get_mut(this_ref)?;
    obj.string_value = Some(String::new());
    Ok(None)
}

/// `Character.digit(char, int)int` — numeric value of char in given radix, or -1.
 // signature must match NativeHandler
pub(crate) fn native_char_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = match args.first() {
        Some(Slot::Int(n)) => (*n).cast_unsigned(),
        _ => return Ok(Some(Slot::Int(-1))),
    };
    let radix = match args.get(1) {
        Some(Slot::Int(n)) => (*n).cast_unsigned(),
        _ => 10,
    };
    let result = char::from_u32(ch)
        .and_then(|c| c.to_digit(radix))
        .map_or(-1, u32::cast_signed);
    Ok(Some(Slot::Int(result)))
}

pub(crate) fn native_char_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let char_val = |s: &Slot| -> VmResult<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => char_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = char_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

pub(crate) fn native_short_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let short_val = |s: &Slot| -> VmResult<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => short_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = short_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

pub(crate) fn native_short_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i16::MIN),
        i32::from(i16::MAX),
    )?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_short_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i16::MIN), i32::from(i16::MAX))?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_byte_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let byte_val = |s: &Slot| -> VmResult<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => byte_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = byte_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

pub(crate) fn native_byte_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(
        args,
        heap,
        i32::from(i8::MIN),
        i32::from(i8::MAX),
    )?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_byte_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i8::MIN), i32::from(i8::MAX))?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Boolean.parseBoolean(String)` — case-insensitive "true" → 1, else 0.
pub(crate) fn native_boolean_parseboolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap.get(*r)?.string_value.clone().unwrap_or_default();
            let val = s.eq_ignore_ascii_case("true");
            Ok(Some(Slot::Int(i32::from(val))))
        }
        Some(Slot::Reference(None)) => Ok(Some(Slot::Int(0))),
        _ => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

pub(crate) fn native_boolean_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let bool_val = |s: &Slot| -> VmResult<bool> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n != 0),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => bool_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = bool_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

pub(crate) fn native_boolean_booleanvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_boolean_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Boolean".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(i32::from(val != 0));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Float.parseFloat(String)` — parses string to float.
pub(crate) fn native_float_parsefloat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f32 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Float(val)))
}

pub(crate) fn native_float_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let float_val = |s: &Slot| -> VmResult<f32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Float(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => float_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = float_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}

pub(crate) fn native_float_floatvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_float_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_float_arg(args, 0)?;
    let r = heap.allocate("java/lang/Float".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Float(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Double.doubleValue()` — unboxes Double to double.
pub(crate) fn native_double_doublevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

/// Native: `Double.valueOf(double)` — boxes double into Double object.
pub(crate) fn native_double_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_double_arg(args, 0)?;
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Double(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Double.parseDouble(String)` — parses string to double.
pub(crate) fn native_double_parsedouble(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f64 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Double(val)))
}

/// Native: `Long.compareTo(Object)` — compares two boxed Longs.
pub(crate) fn native_long_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let long_val = |s: &Slot| -> VmResult<i64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Long(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => long_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = long_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Long.compareUnsigned(long,long)` — compares longs as unsigned 64-bit values.
pub(crate) fn native_long_compareunsigned_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = u64::from_ne_bytes(extract_long_arg(args, 0)?.to_ne_bytes());
    let b = u64::from_ne_bytes(extract_long_arg(args, 1)?.to_ne_bytes());
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Long.toBinaryString(long)` — unsigned binary string.
pub(crate) fn native_long_tobinarystring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:b}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.toOctalString(long)` — unsigned octal string.
pub(crate) fn native_long_tooctalstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:o}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.toHexString(long)` — unsigned lowercase hex string.
pub(crate) fn native_long_tohexstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => u64::from_ne_bytes(v.to_ne_bytes()),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(format!("{val:x}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.toString(long)` — static, converts long to String.
pub(crate) fn native_long_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.intValue()I` — returns the long value narrowed to int.
pub(crate) fn native_long_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = match heap.get(this_ref)?.fields.first() {
         // enum ordinals fit i32
        Some(Slot::Long(v)) => *v as i32,
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(val)))
}

/// Native: `Long.longValue()` — unboxes Long to long.
pub(crate) fn native_long_longvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

/// Native: `Long.decode(String)` — parses prefixed string and boxes long.
pub(crate) fn native_long_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_decode_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.valueOf(String,int)` — parses and boxes long with radix.
pub(crate) fn native_long_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.valueOf(String)` — parses and boxes long.
pub(crate) fn native_long_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.valueOf(long)` — boxes long into Long object.
pub(crate) fn native_long_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.parseLong(String,int)` — parses string to long with radix.
pub(crate) fn native_long_parselong_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_from_string_and_radix_args(args, heap)?;
    Ok(Some(Slot::Long(val)))
}

/// Native: `Long.parseLong(String)` — parses string to long.
pub(crate) fn native_long_parselong(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i64_from_string_arg(args, heap)?;
    Ok(Some(Slot::Long(val)))
}

/// Native: `System.arraycopy(Object src, int srcPos, Object dst, int dstPos, int length)`.
/// Copies `length` elements from `src` starting at `srcPos` into `dst` starting at `dstPos`.
pub(crate) fn native_system_arraycopy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let src_pos = extract_int_arg(args, 1)?;
    let dst_ref = match args.get(2) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let dst_pos = extract_int_arg(args, 3)?;
    let length = extract_int_arg(args, 4)?;
    if length < 0 || src_pos < 0 || dst_pos < 0 {
        return Err(VmError::NegativeArraySize {
            size: length.min(src_pos).min(dst_pos),
        });
    }
    let src_pos = src_pos as usize;
    let dst_pos = dst_pos as usize;
    let length = length as usize;
    // Copy elements one by one to support src == dst (overlapping ranges handled via clone).
    let src_len = heap.get(src_ref)?.fields.len();
    if src_pos + length > src_len {
        return Err(VmError::ArrayIndexOutOfBounds {
            index: i32::try_from(src_pos + length - 1).unwrap_or(i32::MAX),
            length: src_len,
        });
    }
    let src_elems: Vec<Slot> = heap.get(src_ref)?.fields[src_pos..src_pos + length].to_vec();
    let dst_len = heap.get(dst_ref)?.fields.len();
    if dst_pos + length > dst_len {
        return Err(VmError::ArrayIndexOutOfBounds {
            index: i32::try_from(dst_pos + length - 1).unwrap_or(i32::MAX),
            length: dst_len,
        });
    }
    let dst_fields = &mut heap.get_mut(dst_ref)?.fields;
    for (i, slot) in src_elems.into_iter().enumerate() {
        dst_fields[dst_pos + i] = slot;
    }
    Ok(None)
}

/// Native: `Math.round(float)` — rounds float to nearest int.
pub(crate) fn native_math_round_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_float_arg(args, 0)?;
    Ok(Some(Slot::Int(a.round() as i32)))
}

/// Native: `Math.floorDiv(int, int)` — largest int ≤ quotient.
pub(crate) fn native_math_floor_div_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    if b == 0 {
        return Err(VmError::DivisionByZero);
    }
    Ok(Some(Slot::Int(
        a.div_euclid(b) - i32::from(a.wrapping_rem(b) != 0 && (a < 0) != (b < 0)),
    )))
}

/// Native: `Math.hypot(double, double)` — sqrt(x²+y²) without overflow.
pub(crate) fn native_math_hypot(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let x = extract_double_arg(args, 0)?;
    let y = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(x.hypot(y))))
}

/// Native: `Math.cbrt(double)` — cube root.
pub(crate) fn native_math_cbrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.cbrt())))
}

/// Native: `Math.toDegrees(double)` — converts radians to degrees.
pub(crate) fn native_math_to_degrees(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.to_degrees())))
}

/// Native: `Math.toRadians(double)` — converts degrees to radians.
pub(crate) fn native_math_to_radians(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.to_radians())))
}

/// Native: `Math.signum(float)` — sign of a as float: -1.0, 0.0, or 1.0.
pub(crate) fn native_math_signum_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_float_arg(args, 0)?;
    Ok(Some(Slot::Float(a.signum())))
}

/// Native: `Math.signum(double)` — sign of a: -1.0, 0.0, or 1.0.
pub(crate) fn native_math_signum_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.signum())))
}

/// Native: `Math.exp(double)` — Euler's number raised to the given power.
pub(crate) fn native_math_exp(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.exp())))
}

/// Native: `Math.log10(double)` — base-10 logarithm.
pub(crate) fn native_math_log10(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.log10())))
}

/// Native: `Math.log(double)` — natural logarithm.
pub(crate) fn native_math_log(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.ln())))
}

/// Native: `Math.atan2(double, double)` — angle of vector (y, x) in [-π, π].
pub(crate) fn native_math_atan2(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let y = extract_double_arg(args, 0)?;
    let x = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(y.atan2(x))))
}

/// Native: `Math.atan(double)` — arc tangent, result in [-π/2, π/2].
pub(crate) fn native_math_atan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.atan())))
}

/// Native: `Math.acos(double)` — arc cosine, result in [0, π].
pub(crate) fn native_math_acos(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.acos())))
}

/// Native: `Math.asin(double)` — arc sine, result in [-π/2, π/2].
pub(crate) fn native_math_asin(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.asin())))
}

/// Native: `Math.tan(double)` — tangent (argument in radians).
pub(crate) fn native_math_tan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.tan())))
}

/// Native: `Math.cos(double)` — cosine (argument in radians).
pub(crate) fn native_math_cos(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.cos())))
}

/// Native: `Math.sin(double)` — sine (argument in radians).
pub(crate) fn native_math_sin(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.sin())))
}

/// Native: `Math.min(double, double)` — returns the smaller value.
pub(crate) fn native_math_min_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.min(b))))
}

/// Native: `Math.max(double, double)` — returns the larger value.
pub(crate) fn native_math_max_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.max(b))))
}

/// Native: `Math.min(long, long)` — returns the smaller value.
pub(crate) fn native_math_min_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.min(b))))
}

/// Native: `Math.max(long, long)` — returns the larger value.
pub(crate) fn native_math_max_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.max(b))))
}

/// Native: `Math.abs(double)` — returns absolute value.
pub(crate) fn native_math_abs_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.abs())))
}

/// Native: `Math.abs(long)` — returns absolute value.
pub(crate) fn native_math_abs_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Long(a.wrapping_abs())))
}

/// Native: `Math.round(double)` — returns closest long.
pub(crate) fn native_math_round_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Long(a.round() as i64)))
}

/// Native: `Math.ceil(double)` — returns ceiling value.
pub(crate) fn native_math_ceil(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.ceil())))
}

/// Native: `Math.floor(double)` — returns floor value.
pub(crate) fn native_math_floor(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.floor())))
}

/// Native: `Math.pow(double, double)` — returns a raised to the power b.
pub(crate) fn native_math_pow(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    let b = extract_double_arg(args, 1)?;
    Ok(Some(Slot::Double(a.powf(b))))
}

/// Native: `Math.sqrt(double)` — returns square root.
pub(crate) fn native_math_sqrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_double_arg(args, 0)?;
    Ok(Some(Slot::Double(a.sqrt())))
}

/// Native: `Math.floorMod(int, int)` — remainder with the divisor's sign.
pub(crate) fn native_math_floor_mod_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    if b == 0 {
        return Err(VmError::DivisionByZero);
    }
    let remainder = a.wrapping_rem(b);
    let floor_mod = if remainder != 0 && (remainder < 0) != (b < 0) {
        remainder.wrapping_add(b)
    } else {
        remainder
    };
    Ok(Some(Slot::Int(floor_mod)))
}

/// Native: `Math.abs(int)` — returns absolute value.
pub(crate) fn native_math_abs_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(a.wrapping_abs())))
}

/// Native: `Math.min(int, int)` — returns the smaller value.
pub(crate) fn native_math_min_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.min(b))))
}

/// Native: `Math.max(int, int)` — returns the larger value.
pub(crate) fn native_math_max_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.max(b))))
}

/// Native: `String.toString()` — identity, returns `this`.
pub(crate) fn native_string_tostring(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(args.first().copied().unwrap_or(Slot::Reference(None))))
}

/// Native: `String.hashCode()` — Java's hash algorithm: `s[0]*31^(n-1) + s[1]*31^(n-2) + ... + s[n-1]`.
pub(crate) fn native_string_hashcode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let mut h: i32 = 0;
    for ch in s.chars() {
        h = h.wrapping_mul(31).wrapping_add(ch as i32);
    }
    Ok(Some(Slot::Int(h)))
}

/// Native: `String.split(String, int)` — split with a limit parameter.
pub(crate) fn native_string_split_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let delim_ref = extract_ref_arg(args, 1)?;
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let limit = match args.get(2) {
        Some(Slot::Int(n)) => *n,
        _ => 0,
    };
     // limit is validated > 0 before the cast
    let parts: Vec<String> = if delim.is_empty() {
        let chars: Vec<String> = s.chars().map(|c| c.to_string()).collect();
        if limit > 0 && (limit as usize) < chars.len() {
            let mut v = chars[..limit as usize - 1].to_vec();
            v.push(chars[limit as usize - 1..].join(""));
            v
        } else {
            chars
        }
    } else {
        let re = regex::Regex::new(&delim)
            .unwrap_or_else(|_| regex::Regex::new(&regex::escape(&delim)).unwrap());
        if limit > 0 {
            re.splitn(&s, limit as usize).map(str::to_string).collect()
        } else {
            let mut v: Vec<String> = re.split(&s).map(str::to_string).collect();
            if limit == 0 {
                while v.last().is_some_and(String::is_empty) {
                    v.pop();
                }
            }
            v
        }
    };
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (i, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[i] = Slot::Reference(Some(str_ref));
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `String.split(String)` — splits string by delimiter, returns String array.
pub(crate) fn native_string_split(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let delim_ref = extract_ref_arg(args, 1)?;
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // Use regex split (Java's String.split uses regex); remove trailing empty strings
    // to match Java's default split behaviour.
    // Special case: split("") splits into individual chars (Java 21 semantics — no leading "").
    let parts: Vec<String> = if delim.is_empty() {
        s.chars().map(|c| c.to_string()).collect()
    } else {
        regex::Regex::new(&delim).map_or_else(
            |_| s.split(delim.as_str()).map(str::to_string).collect(),
            |re| {
                let mut v: Vec<String> = re.split(&s).map(str::to_string).collect();
                while v.last().is_some_and(String::is_empty) {
                    v.pop();
                }
                v
            },
        )
    };
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (i, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string(part.clone());
        heap.get_mut(arr_ref)?.fields[i] = Slot::Reference(Some(str_ref));
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `String.replace(CharSequence, CharSequence)` — replaces all occurrences of target with replacement.
pub(crate) fn native_string_replace_charsequence(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let target_ref = extract_ref_arg(args, 1)?;
    let target = heap
        .get(target_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let replacement_ref = extract_ref_arg(args, 2)?;
    let replacement = heap
        .get(replacement_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let result = s.replace(&*target, &replacement);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replace(char, char)` — replaces all occurrences of old char with new char.
pub(crate) fn native_string_replace_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let old_char = char::from_u32(extract_int_arg(args, 1)?.cast_unsigned()).unwrap_or('?');
    let new_char = char::from_u32(extract_int_arg(args, 2)?.cast_unsigned()).unwrap_or('?');
    let result = s.replace(old_char, &new_char.to_string());
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.toLowerCase()` — returns a new lowercase String.
pub(crate) fn native_string_tolowercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_lowercase());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.toUpperCase()` — returns a new uppercase String.
pub(crate) fn native_string_touppercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_uppercase());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;`
pub(crate) fn native_string_format(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fmt_ref = extract_ref_arg(args, 0)?;
    let fmt = heap.get(fmt_ref)?.string_value.clone().unwrap_or_default();

    let arr_len = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.fields.len(),
        _ => 0,
    };

    let mut result = String::new();
    let mut arg_idx = 0usize;
    let mut chars = fmt.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '%' {
            result.push(ch);
            continue;
        }

        // Parse flags: -, +, 0
        let mut flags = String::new();
        while let Some(&f) = chars.peek() {
            if matches!(f, '-' | '+' | '0' | ' ' | '#') {
                flags.push(f);
                chars.next();
            } else {
                break;
            }
        }

        // Parse optional width
        let mut width_str = String::new();
        while let Some(&d) = chars.peek() {
            if d.is_ascii_digit() {
                width_str.push(d);
                chars.next();
            } else {
                break;
            }
        }
        let width: Option<usize> = if width_str.is_empty() {
            None
        } else {
            width_str.parse().ok()
        };

        // Parse optional precision: .N
        let precision: Option<usize> = if chars.peek() == Some(&'.') {
            chars.next();
            let mut prec_str = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    prec_str.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            prec_str.parse().ok()
        } else {
            None
        };

        let Some(spec) = chars.next() else {
            break;
        };

        match spec {
            '%' => result.push('%'),
            'n' => result.push('\n'),
            's' | 'd' | 'f' | 'x' | 'X' | 'b' | 'c' | 'o' | 'e' | 'E' => {
                let slot = if arg_idx < arr_len {
                    match args.get(1) {
                        Some(Slot::Reference(Some(r))) => heap
                            .get(*r)?
                            .fields
                            .get(arg_idx)
                            .copied()
                            .unwrap_or(Slot::Reference(None)),
                        _ => Slot::Reference(None),
                    }
                } else {
                    Slot::Reference(None)
                };
                arg_idx += 1;
                let formatted = format_arg(spec, &flags, width, precision, &slot, heap)?;
                result.push_str(&formatted);
            }
            _ => {
                result.push('%');
                result.push(spec);
            }
        }
    }

    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.concat(String)` — concatenates two strings.
pub(crate) fn native_string_concat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s1 = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let other_ref = extract_ref_arg(args, 1)?;
    let s2 = heap
        .get(other_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let r = heap.allocate_string(format!("{s1}{s2}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(Object)` — converts Object to String.
pub(crate) fn native_string_value_of_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap_object_to_string(heap.get(*r)?, *r);
            let r = heap.allocate_string(s);
            Ok(Some(Slot::Reference(Some(r))))
        }
        Some(Slot::Reference(None)) => {
            let r = heap.allocate_string("null".to_string());
            Ok(Some(Slot::Reference(Some(r))))
        }
        _ => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

/// Native: `String.valueOf(char)` — converts char to String.
pub(crate) fn native_string_value_of_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(char)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(boolean)` — converts boolean to String.
pub(crate) fn native_string_value_of_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v != 0,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(boolean)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(if val { "true" } else { "false" }.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(float)` — converts float to String.
pub(crate) fn native_string_value_of_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_float_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(double)` — converts double to String.
pub(crate) fn native_string_value_of_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_double_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(long)` — converts long to String.
pub(crate) fn native_string_value_of_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_long_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(char[])String` — creates String from char array.
pub(crate) fn native_string_value_of_char_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => {
            return Ok(Some(Slot::Reference(Some(
                heap.allocate_string(String::new()),
            ))));
        }
    };
    let chars: String = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| match s {
            Slot::Int(n) => char::from_u32(u32::try_from(*n).unwrap_or(0)),
            _ => None,
        })
        .collect();
    Ok(Some(Slot::Reference(Some(heap.allocate_string(chars)))))
}

/// Native: `String.<init>(char[])V` — constructs a String from a char array.
pub(crate) fn native_string_init_from_chars(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(arr_ref))) = args.get(1).copied() else {
        heap.get_mut(this_ref)?.string_value = Some(String::new());
        return Ok(None);
    };
    let chars: String = heap
        .get(arr_ref)?
        .fields
        .iter()
        .filter_map(|s| match s {
            Slot::Int(n) => char::from_u32(u32::try_from(*n).unwrap_or(0)),
            _ => None,
        })
        .collect();
    heap.get_mut(this_ref)?.string_value = Some(chars);
    Ok(None)
}

/// Native: `Objects.hashCode(Object)I` — returns 0 for null, else object identity hash.
pub(crate) fn native_objects_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let hash = match args.first() {
        Some(Slot::Reference(Some(r))) => (*r & 0x7FFF_FFFF) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(hash)))
}

/// Native: `Objects.toString(Object, String)String` — returns nullDefault if null, else toString.
pub(crate) fn native_objects_tostring_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => {
            // null → return the default string (args[1])
            let default_ref = match args.get(1) {
                Some(Slot::Reference(Some(r))) => *r,
                _ => heap.allocate_string("null".to_string()),
            };
            Ok(Some(Slot::Reference(Some(default_ref))))
        }
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            let text = obj
                .string_value
                .as_deref()
                .map_or_else(|| format!("{}@{}", obj.class_name, r), str::to_owned);
            let _ = obj;
            let s = heap.allocate_string(text);
            Ok(Some(Slot::Reference(Some(s))))
        }
        Some(Slot::Int(n)) => {
            let s = heap.allocate_string(n.to_string());
            Ok(Some(Slot::Reference(Some(s))))
        }
        Some(other) => {
            let s = heap.allocate_string(format!("{other:?}"));
            Ok(Some(Slot::Reference(Some(s))))
        }
    }
}

/// Native: `Objects.toString(Object)` — returns `"null"` if null, else `string_value` or class name.
pub(crate) fn native_objects_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let s = match args.first() {
        Some(Slot::Reference(None)) | None => heap.allocate_string("null".to_string()),
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            let text = obj
                .string_value
                .as_deref()
                .map_or_else(|| format!("{}@{}", obj.class_name, r), str::to_owned);
            let _ = obj;
            heap.allocate_string(text)
        }
        Some(Slot::Int(n)) => heap.allocate_string(n.to_string()),
        Some(Slot::Long(n)) => heap.allocate_string(n.to_string()),
        Some(other) => heap.allocate_string(format!("{other:?}")),
    };
    Ok(Some(Slot::Reference(Some(s))))
}

/// Native: `Objects.equals(Object, Object)Z` — null-safe equality check.
pub(crate) fn native_objects_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = args.first().copied().unwrap_or(Slot::Reference(None));
    let b = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let equal = slots_equal(&a, &b, heap);
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Objects.requireNonNull(Object, String)Object` — throws NPE with message if null.
pub(crate) fn native_objects_require_non_null_msg(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => Err(VmError::NullPointerException),
        Some(s) => Ok(Some(*s)),
    }
}

/// Native: `Objects.requireNonNull(Object)Object` — throws NPE if null, else returns arg.
pub(crate) fn native_objects_require_non_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(None)) | None => Err(VmError::NullPointerException),
        Some(s) => Ok(Some(*s)),
    }
}

/// Native: `Objects.nonNull(Object)Z` — returns 1 if argument is not null.
pub(crate) fn native_objects_non_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let is_null = matches!(args.first(), Some(Slot::Reference(None)) | None);
    Ok(Some(Slot::Int(i32::from(!is_null))))
}

/// Native: `Objects.isNull(Object)Z` — returns 1 if argument is null.
pub(crate) fn native_objects_is_null(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let is_null = matches!(args.first(), Some(Slot::Reference(None)) | None);
    Ok(Some(Slot::Int(i32::from(is_null))))
}

/// Native: `Long.sum(long, long)` — static addition (functional interface target).
pub(crate) fn native_long_sum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Long(a.wrapping_add(b))))
}

/// Native: `Long.compare(long, long)` — static two-value comparison.
pub(crate) fn native_long_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_long_arg(args, 0)?;
    let b = extract_long_arg(args, 1)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Long.signum(long)` — returns -1, 0, or 1.
pub(crate) fn native_long_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum() as i32)))
}

/// Native: `Long.reverseBytes(long)` — reverse byte order (swap endianness).
pub(crate) fn native_long_reverse_bytes(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
        let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Long(v.swap_bytes() as i64)))
}

/// Native: `Long.reverse(long)` — reverse bit order.
pub(crate) fn native_long_reverse(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
        let v = extract_long_arg(args, 0)? as u64;
    Ok(Some(Slot::Long(v.reverse_bits() as i64)))
}

/// Native: `Long.lowestOneBit(long)` — return value with only the lowest set bit.
pub(crate) fn native_long_lowest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_long_arg(args, 0)?;
    Ok(Some(Slot::Long(v & v.wrapping_neg())))
}

/// Native: `Long.highestOneBit(long)` — return value with only the highest set bit.
pub(crate) fn native_long_highest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
        let v = extract_long_arg(args, 0)? as u64;
    let result = if v == 0 { 0u64 } else { 1u64 << v.ilog2() };
    Ok(Some(Slot::Long(result as i64)))
}

pub(crate) fn native_long_trailing_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
        let v = extract_long_arg(args, 0)? as u64;
        Ok(Some(Slot::Int(v.trailing_zeros() as i32)))
}

pub(crate) fn native_long_leading_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
        let v = extract_long_arg(args, 0)? as u64;
        Ok(Some(Slot::Int(v.leading_zeros() as i32)))
}

pub(crate) fn native_long_bitcount(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
        let v = extract_long_arg(args, 0)? as u64;
        Ok(Some(Slot::Int(v.count_ones() as i32)))
}

/// Native: `Integer.min(int, int)` — static min (functional interface target).
pub(crate) fn native_integer_min_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.min(b))))
}

/// Native: `Integer.max(int, int)` — static max (functional interface target).
pub(crate) fn native_integer_max_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.max(b))))
}

/// Native: `Integer.sum(int, int)` — static addition (functional interface target).
pub(crate) fn native_integer_sum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(a.wrapping_add(b))))
}

/// Native: `Integer.compare(int, int)` — static two-value comparison.
pub(crate) fn native_integer_compare_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = extract_int_arg(args, 0)?;
    let b = extract_int_arg(args, 1)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Integer.signum(int)` — returns -1, 0, or 1.
pub(crate) fn native_integer_signum(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(v.signum())))
}

/// Native: `Integer.reverseBytes(int)` — reverse byte order (swap endianness).
pub(crate) fn native_integer_reverse_bytes(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.swap_bytes() as i32)))
}

/// Native: `Integer.reverse(int)` — reverse bit order.
pub(crate) fn native_integer_reverse(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    Ok(Some(Slot::Int(v.reverse_bits() as i32)))
}

/// Native: `Integer.lowestOneBit(int)` — return value with only the lowest set bit.
pub(crate) fn native_integer_lowest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)?;
    Ok(Some(Slot::Int(v & v.wrapping_neg())))
}

/// Native: `Integer.highestOneBit(int)` — return value with only the highest set bit.
pub(crate) fn native_integer_highest_one_bit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
    let result = if v == 0 { 0u32 } else { 1u32 << v.ilog2() };
    Ok(Some(Slot::Int(result as i32)))
}

/// Native: `Integer.numberOfTrailingZeros(int)` — count trailing zero bits.
pub(crate) fn native_integer_trailing_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
        Ok(Some(Slot::Int(v.trailing_zeros() as i32)))
}

/// Native: `Integer.numberOfLeadingZeros(int)` — count leading zero bits.
pub(crate) fn native_integer_leading_zeros(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
        Ok(Some(Slot::Int(v.leading_zeros() as i32)))
}

/// Native: `Integer.bitCount(int)` — count number of set bits (popcount).
pub(crate) fn native_integer_bitcount(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let v = extract_int_arg(args, 0)? as u32;
        Ok(Some(Slot::Int(v.count_ones() as i32)))
}

/// Native: `Integer.compareTo(Object)` — compares two boxed Integers.
pub(crate) fn native_integer_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let int_val = |s: &Slot| -> VmResult<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => int_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = int_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Integer.compareUnsigned(int,int)` — compares ints as unsigned 32-bit values.
pub(crate) fn native_integer_compareunsigned_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let b = u32::from_ne_bytes(extract_int_arg(args, 1)?.to_ne_bytes());
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

/// Native: `Integer.toUnsignedLong(int)` — widen via unsigned 32-bit interpretation.
pub(crate) fn native_integer_tounsignedlong_static(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    Ok(Some(Slot::Long(i64::from(val))))
}

/// Native: `Integer.toBinaryString(int)` — unsigned binary string.
pub(crate) fn native_integer_tobinarystring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:b}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.toOctalString(int)` — unsigned octal string.
pub(crate) fn native_integer_tooctalstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:o}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.toHexString(int)` — unsigned lowercase hex string.
pub(crate) fn native_integer_tohexstring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = u32::from_ne_bytes(extract_int_arg(args, 0)?.to_ne_bytes());
    let r = heap.allocate_string(format!("{val:x}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.toString(int)` — static, converts int to String.
pub(crate) fn native_integer_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.intValue()` — unboxes Integer to int.
pub(crate) fn native_integer_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

/// Native: `Integer.decode(String)` — parses prefixed string and boxes int.
pub(crate) fn native_integer_decode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_i32_decode_from_string_arg(args, heap)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.valueOf(String,int)` — parses and boxes int with radix.
pub(crate) fn native_integer_valueof_string_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(args, heap, i32::MIN, i32::MAX)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.valueOf(String)` — parses and boxes int.
pub(crate) fn native_integer_valueof_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.valueOf(int)` — boxes int into Integer object.
pub(crate) fn native_integer_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r)?.fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.parseInt(String,int)` — parses string to int with radix.
pub(crate) fn native_integer_parseint_radix(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_and_radix_args(args, heap, i32::MIN, i32::MAX)?;
    Ok(Some(Slot::Int(val)))
}

/// Native: `Integer.parseInt(String)` — parses string to int.
pub(crate) fn native_integer_parseint(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = parse_bounded_i32_from_string_arg(args, heap, i32::MIN, i32::MAX)?;
    Ok(Some(Slot::Int(val)))
}

/// Native: `String.toCharArray()` — convert string to char array.
pub(crate) fn native_string_tochararray(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let chars: Vec<char> = s.chars().collect();
    let arr_ref = heap.allocate("[C".to_string(), chars.len());
    for (i, &c) in chars.iter().enumerate() {
        heap.get_mut(arr_ref)?.fields[i] = Slot::Int(c as i32);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `String.trim()` — remove leading and trailing whitespace.
pub(crate) fn native_string_trim(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let trimmed = s.trim().to_string();
    let r = heap.allocate_string(trimmed);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.endsWith(String)` — check if string ends with suffix.
pub(crate) fn native_string_endswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let suffix_ref = extract_ref_arg(args, 1)?;
    let suffix = heap
        .get(suffix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.ends_with(&suffix)))))
}

/// Native: `String.startsWith(String)` — check if string starts with prefix.
pub(crate) fn native_string_startswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let prefix_ref = extract_ref_arg(args, 1)?;
    let prefix = heap
        .get(prefix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.starts_with(&prefix)))))
}

/// Native: `String.compareTo(Object)` — lexicographic comparison via Object descriptor.
pub(crate) fn native_string_compareto_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_val = |s: &Slot| -> VmResult<String> {
        match s {
            Slot::Reference(Some(r)) => Ok(heap.get(*r)?.string_value.clone().unwrap_or_default()),
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => str_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = str_val(args.get(1).ok_or(VmError::NullPointerException)?)?;
    Ok(Some(Slot::Int(ordering_to_int(a.as_str().cmp(b.as_str())))))
}

/// Native: `String.compareTo(String)` — delegates to the Object overload.
pub(crate) fn native_string_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_string_compareto_object(args, heap, out, control)
}

/// Native: `String.isEmpty()` — check if string is empty.
pub(crate) fn native_string_isempty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.is_empty()))))
}

/// Native: `String.contains(CharSequence)` — check if string contains target.
pub(crate) fn native_string_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let target_ref = extract_ref_arg(args, 1)?;
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.contains(target)))))
}

/// Native: `String.lastIndexOf(String, int)I` — last occurrence at or before fromIndex.
pub(crate) fn native_string_last_indexof_from(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let sub_ref = extract_ref_arg(args, 1)?;
     // .max(0) guarantees non-negative
    let from = extract_int_arg(args, 2).unwrap_or(0).max(0) as usize;
    let this_str = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let sub_str = heap.get(sub_ref)?.string_value.clone().unwrap_or_default();
    let search_in = if from + sub_str.len() < this_str.len() {
        &this_str[..from + sub_str.len()]
    } else {
        &this_str
    };
        let result = search_in.rfind(sub_str.as_str()).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.indexOf(String, int)I` — first occurrence at or after fromIndex.
pub(crate) fn native_string_indexof_from(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target_ref = extract_ref_arg(args, 1)?;
     // .max(0) guarantees non-negative
    let from = extract_int_arg(args, 2).unwrap_or(0).max(0) as usize;
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();
    let search_in = if from < s.len() { &s[from..] } else { "" };
        let result = search_in.find(target).map_or(-1, |i| (from + i) as i32);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.indexOf(String)` — find first occurrence of target.
pub(crate) fn native_string_indexof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let target_ref = extract_ref_arg(args, 1)?;
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();

        let result = s.find(target).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.substring(int, int)` — substring from begin to end (exclusive).
pub(crate) fn native_string_substring_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let begin = extract_int_arg(args, 1)? as usize;
    let end = extract_int_arg(args, 2)? as usize;
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        if begin > end || end > s.len() {
            return Err(VmError::ArrayIndexOutOfBounds {
                index: i32::try_from(end).unwrap_or(i32::MAX),
                length: s.len(),
            });
        }
        s.chars().skip(begin).take(end - begin).collect::<String>()
    };

    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.substring(int)` — substring from begin to end.
pub(crate) fn native_string_substring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;

    let begin = extract_int_arg(args, 1)? as usize;
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        if begin > s.len() {
            return Err(VmError::ArrayIndexOutOfBounds {
                index: i32::try_from(begin).unwrap_or(i32::MAX),
                length: s.len(),
            });
        }
        s.chars().skip(begin).collect::<String>()
    };

    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_thread_sleep(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let millis = match args.first() {
        Some(Slot::Long(value)) => *value,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "long",
                got: "other",
            });
        }
    };
    let millis = u64::try_from(millis.max(0)).unwrap_or(0);
    control.request(NativeThreadAction::Sleep(std::time::Duration::from_millis(
        millis,
    )));
    Ok(None)
}

pub(crate) fn native_thread_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    let thread = heap.get(thread_ref)?;
    let Some(Slot::Int(thread_id)) = thread.fields.get(THREAD_ID_SLOT) else {
        return Ok(None);
    };
    if *thread_id >= 0 {
        control.request(NativeThreadAction::Join {
            thread_id: *thread_id,
        });
    }
    Ok(None)
}

pub(crate) fn native_thread_start(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let thread_ref = extract_ref_arg(args, 0)?;
    control.request(NativeThreadAction::Start { thread_ref });
    Ok(None)
}

pub(crate) fn native_thread_init_runnable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let target = extract_slot_arg(args, 1);
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = target;
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    Ok(None)
}

pub(crate) fn native_thread_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    Ok(None)
}

pub(crate) fn native_thread_current_thread(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let thread_ref = heap.allocate("java/lang/Thread".to_string(), 2);
    let thread = heap.get_mut(thread_ref)?;
    thread.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    thread.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    Ok(Some(Slot::Reference(Some(thread_ref))))
}

 // must match NativeHandler signature
pub(crate) fn native_system_nano_time(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Long(monotonic_nano_time_now())))
}

 // must match NativeHandler signature
pub(crate) fn native_system_current_time_millis(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Long(system_time_to_epoch_millis(
        std::time::SystemTime::now(),
    ))))
}

/// Native: `System.identityHashCode(Object)I` — returns a stable identity hash (heap address low bits).
pub(crate) fn native_system_identity_hash_code(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
        let hash = match args.first() {
        Some(Slot::Reference(Some(r))) => (*r & 0x7FFF_FFFF) as i32,
        _ => 0,
    };
    Ok(Some(Slot::Int(hash)))
}

/// Native: `System.lineSeparator()String` — returns the platform line separator.
pub(crate) fn native_system_line_separator(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let r = heap.allocate_string("\n".to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_system_get_property_with_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = system_property_value(&key).map_or_else(
        || extract_slot_arg(args, 1),
        |value| Slot::Reference(Some(heap.allocate_string(value))),
    );
    Ok(Some(result))
}

pub(crate) fn native_system_get_property(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let key_ref = extract_ref_arg(args, 0)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = system_property_value(&key).map_or(Slot::Reference(None), |value| {
        Slot::Reference(Some(heap.allocate_string(value)))
    });
    Ok(Some(result))
}

pub(crate) fn native_system_exit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let code = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 1,
    };
    Err(VmError::SystemExit { code })
}

/// Native: `String.valueOf(int)` — static method, returns string of int.
pub(crate) fn native_string_value_of_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let s = val.to_string();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_zero_long(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Long(0)))
}

pub(crate) fn native_false_boolean(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Int(0)))
}

/// Native: `ReverseOrderComparator.compare(O,O)I` — negates natural order.
pub(crate) fn native_reverse_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let result = native_natural_order_compare(args, heap, out, control, ops)?;
    Ok(Some(match result {
        Some(Slot::Int(v)) => Slot::Int(-v),
        other => other.unwrap_or(Slot::Int(0)),
    }))
}

/// Native: `NaturalOrderComparator.compare(O,O)I` — delegates to `o1.compareTo(o2)`.
pub(crate) fn native_natural_order_compare(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // args: [this, o1, o2]
    let o1 = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let o2 = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let o1_class = match &o1 {
        Slot::Reference(Some(r)) => heap.get(*r)?.class_name.clone(),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let result = ops.invoke(
        heap,
        out,
        &o1_class,
        "compareTo",
        "(Ljava/lang/Object;)I",
        vec![o1, o2],
    )?;
    let _ = control;
    Ok(Some(result.unwrap_or(Slot::Int(0))))
}

/// Native: `String.<init>(String)V` — copy constructor: copies `string_value` from source.
pub(crate) fn native_string_init_copy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let src_val = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = src_val;
    Ok(None)
}

/// Native: `Throwable.toString()String` — returns `"ClassName: message"` or just class name.
pub(crate) fn native_throwable_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let class_name = obj.class_name.replace('/', ".");
    let s = match &obj.string_value {
        Some(msg) => format!("{class_name}: {msg}"),
        None => class_name,
    };
    let _ = obj;
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Throwable.<init>(String, Throwable)V` — stores message + cause.
pub(crate) fn native_throwable_init_string_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msg = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = msg;
    // Store cause in fields[0] (Throwable.cause field)
    if let Some(&cause_slot) = args.get(2)
        && let Ok(obj) = heap.get_mut(this_ref)
        && !obj.fields.is_empty()
    {
        obj.fields[0] = cause_slot;
    }
    Ok(None)
}

/// Native: `Throwable.<init>(String)V` — stores detail message in `string_value`.
pub(crate) fn native_throwable_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let msg = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone(),
        _ => None,
    };
    heap.get_mut(this_ref)?.string_value = msg;
    Ok(None)
}

/// Native: `Object.clone()` — shallow-copies a heap object.
pub(crate) fn native_object_clone(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let cloned_class = obj.class_name.clone();
    let cloned_fields = obj.fields.clone();
    let cloned_string = obj.string_value.clone();
    let new_ref = heap.allocate(cloned_class, 0);
    let dest = heap.get_mut(new_ref)?;
    dest.fields = cloned_fields;
    dest.string_value = cloned_string;
    Ok(Some(Slot::Reference(Some(new_ref))))
}

/// Native: `Object.toString()` — delegates to `heap_object_to_string` so String,
/// boxed primitives, and opaque objects all produce the correct Java representation.
pub(crate) fn native_object_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let s = heap_object_to_string(heap.get(this_ref)?, this_ref);
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Object.hashCode()` — returns heap address as hash.
pub(crate) fn native_object_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
                Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(*r as i32))),
        _ => Err(VmError::NullPointerException),
    }
}

/// Native: `Object.equals(Object)` — default Java object identity comparison.
pub(crate) fn native_object_equals(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let equal = extract_ref_arg(args, 1) == Ok(this_ref);
    Ok(Some(Slot::Int(i32::from(equal))))
}

/// Native: `Object.getClass()` — returns a lightweight `Class` object for the runtime type.
pub(crate) fn native_object_get_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let class_name = heap.get(this_ref)?.class_name.clone();
    let class_ref = allocate_class_object(heap, &class_name)?;
    Ok(Some(Slot::Reference(Some(class_ref))))
}

/// Native: `Object.<init>()V` - root constructor is a no-op after null-checking `this`.
pub(crate) fn native_object_init(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(None)
}

/// Native: `String.charAt(int)` — returns char at index as int.
pub(crate) fn native_string_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let index = extract_int_arg(args, 1)?;
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or("");
    let ch = s
        .chars()
        .nth(index as usize)
        .ok_or(VmError::ArrayIndexOutOfBounds {
            index,
            length: s.len(),
        })?;
    Ok(Some(Slot::Int(ch as i32)))
}

/// Native: `String.equals(Object)` — compares string content.
pub(crate) fn native_string_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let Ok(other_ref) = extract_ref_arg(args, 1) else {
        return Ok(Some(Slot::Int(0)));
    };
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let other_obj = heap.get(other_ref)?;
    let this_str = this_obj.string_value.as_deref().unwrap_or_default();
    let other_str = other_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(this_str == other_str))))
}

/// Native: `String.length()` — returns string length as int.
pub(crate) fn native_string_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    let len = obj.string_value.as_ref().map_or(0, String::len);
    Ok(Some(Slot::Int(len as i32)))
}
