fn pattern_text_and_flags(heap: &duke_gc::Heap, pat_ref: u64) -> Result<(String, i32)> {
    let pat = heap.get(pat_ref)?;
    let pattern_str = pat.string_value.clone().unwrap_or_default();
    let flags = match pat.fields.get(PATTERN_FLAGS_FIELD).copied() {
        Some(Slot::Int(flags)) => flags,
        _ => 0,
    };
    Ok((pattern_str, flags))
}
/// Native: `Pattern.compile(String)Pattern` — static factory.
pub(crate) fn native_pattern_compile(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let pat_str_ref = extract_ref_arg(args, 0)?;
    let pattern_str = heap.get(pat_str_ref)?.string_value.clone().unwrap_or_default();
    compile_java_regex(&pattern_str)?;
    let pat_ref = allocate_pattern(heap, pattern_str, 0)?;
    Ok(Some(Slot::Reference(Some(pat_ref))))
}
/// Native: `Pattern.compile(String,int)Pattern` — static factory with flags.
pub(crate) fn native_pattern_compile_flags(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let pat_str_ref = extract_ref_arg(args, 0)?;
    let flags = extract_int_arg(args, 1)?;
    let pattern_str = heap.get(pat_str_ref)?.string_value.clone().unwrap_or_default();
    compile_java_regex_with_flags(&pattern_str, flags)?;
    let pat_ref = allocate_pattern(heap, pattern_str, flags)?;
    Ok(Some(Slot::Reference(Some(pat_ref))))
}
/// Native: `Pattern.matcher(CharSequence)Matcher` — creates a Matcher.
pub(crate) fn native_pattern_matcher(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_slot = extract_slot_arg(args, 1);
    let m_ref = heap
        .allocate("java/util/regex/Matcher".to_string(), MATCHER_FIELD_COUNT);
    heap.get_mut(m_ref)?.fields[MATCHER_PATTERN_FIELD] = Slot::Reference(Some(pat_ref));
    heap.get_mut(m_ref)?.fields[MATCHER_INPUT_FIELD] = input_slot;
    reset_matcher_fields(heap, m_ref)?;
    Ok(Some(Slot::Reference(Some(m_ref))))
}
/// Native: `Pattern.matches(String,CharSequence)Z` — static full-string match.
pub(crate) fn native_pattern_matches_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap.get(input_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    Ok(Some(Slot::Int(i32::from(result))))
}
fn pattern_split_impl(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    limit: i32,
) -> Result<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = heap.get(input_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let parts = regex_split_parts(&re, &input, limit);
    let arr_ref = alloc_string_array_from_parts(heap, &parts)?;
    Ok(Some(Slot::Reference(Some(arr_ref))))
}
/// Native: `Pattern.split(CharSequence)String[]`.
pub(crate) fn native_pattern_split(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    pattern_split_impl(args, heap, 0)
}
/// Native: `Pattern.split(CharSequence,int)String[]`.
pub(crate) fn native_pattern_split_limit(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    pattern_split_impl(args, heap, extract_int_arg(args, 2)?)
}
