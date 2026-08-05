/// Native: `Pattern.compile(String)Pattern` — static factory.
pub(crate) fn native_pattern_compile(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let pat_str_ref = extract_ref_arg(args, 0)?;
    let pattern_str = string_value_from_ref(heap, pat_str_ref).unwrap_or_default();
    // Validate the regex eagerly so we fail here not at match time.
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
    let pattern_str = string_value_from_ref(heap, pat_str_ref).unwrap_or_default();
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
    // fields: pattern, input, next find position, last match start/end, append position
    let m_ref = heap.allocate("java/util/regex/Matcher".to_string(), MATCHER_FIELD_COUNT);
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
    let pattern_str = string_value_from_ref(heap, pat_ref).unwrap_or_default();
    let input = charsequence_chars(heap, input_ref)?.unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    Ok(Some(Slot::Int(i32::from(result))))
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
/// Native: `Matcher.find()Z` — finds next match; advances position.
pub(crate) fn native_matcher_find(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let (pat_ref, input_ref, pos) = {
        let obj = heap.get(m_ref)?;
        let Some(Slot::Reference(Some(pat_ref))) = obj.fields.get(MATCHER_PATTERN_FIELD).copied() else {
            return Ok(Some(Slot::Int(0)));
        };
        let Some(Slot::Reference(Some(input_ref))) = obj.fields.get(MATCHER_INPUT_FIELD).copied() else {
            return Ok(Some(Slot::Int(0)));
        };
        let pos = match obj.fields.get(MATCHER_POS_FIELD).copied() {
            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
            _ => 0,
        };
        (pat_ref, input_ref, pos)
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = charsequence_chars(heap, input_ref)?.unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    if let Some(m) = re.find_at(&input, pos.min(input.len())) {
        store_matcher_match(heap, m_ref, &input, m.start(), m.end())?;
        Ok(Some(Slot::Int(1)))
    } else {
        set_matcher_no_match(heap, m_ref)?;
        Ok(Some(Slot::Int(0)))
    }
}
/// Native: `Matcher.matches()Z` — full-string match (resets position).
pub(crate) fn native_matcher_matches(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let (pat_ref, input_ref) = {
        let obj = heap.get(m_ref)?;
        let Some(Slot::Reference(Some(pat_ref))) = obj.fields.get(MATCHER_PATTERN_FIELD).copied() else {
            return Ok(Some(Slot::Int(0)));
        };
        let Some(Slot::Reference(Some(input_ref))) = obj.fields.get(MATCHER_INPUT_FIELD).copied() else {
            return Ok(Some(Slot::Int(0)));
        };
        (pat_ref, input_ref)
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = charsequence_chars(heap, input_ref)?.unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let matched = re
        .find(&input)
        .filter(|m| m.start() == 0 && m.end() == input.len());
    if let Some(m) = matched {
        store_matcher_match(heap, m_ref, &input, m.start(), m.end())?;
    } else {
        set_matcher_no_match(heap, m_ref)?;
    }
    Ok(Some(Slot::Int(i32::from(matched.is_some()))))
}
/// Native: `Matcher.group()String` — returns text of last match.
pub(crate) fn native_matcher_group(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let Some(matched) = matcher_group_text(heap, m_ref, MatcherGroup::Index(0))? else {
        return Ok(Some(Slot::Reference(None)));
    };
    let r = heap.allocate_string(matched);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Matcher.group(int)String` — returns the nth capture group from the last match.
pub(crate) fn native_matcher_group_n(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Int(n)) if n >= 0 => usize::try_from(n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(regex_index_out_of_bounds(format!("No group {n}"))),
        _ => 0,
    };
    if n == 0 {
        // group(0) == group() — full match
        return native_matcher_group(args, heap, out, control);
    }
    let Some(group) = matcher_group_text(heap, m_ref, MatcherGroup::Index(n))? else {
        return Ok(Some(Slot::Reference(None)));
    };
    let s = heap.allocate_string(group);
    Ok(Some(Slot::Reference(Some(s))))
}
/// Native: `Matcher.group(String)String` — returns a named capture group.
pub(crate) fn native_matcher_group_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let name = string_value_from_ref(heap, name_ref).unwrap_or_default();
    let Some(group) = matcher_group_text(heap, m_ref, MatcherGroup::Name(&name))? else {
        return Ok(Some(Slot::Reference(None)));
    };
    let s = heap.allocate_string(group);
    Ok(Some(Slot::Reference(Some(s))))
}
/// Native: `Matcher.groupCount()I` — number of capturing groups, excluding group 0.
pub(crate) fn native_matcher_group_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let Some((pattern_str, flags, _)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Ok(Some(Slot::Int(0)));
    };
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let count = re.captures_len().saturating_sub(1);
    Ok(Some(Slot::Int(i32::try_from(count).unwrap_or(i32::MAX))))
}
/// Native: `Matcher.start()I` — start index of last match.
pub(crate) fn native_matcher_start(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let start = matcher_group_bounds_java(heap, m_ref, MatcherGroup::Index(0))?
        .map_or(-1, |(start, _)| i32::try_from(start).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(start)))
}
/// Native: `Matcher.start(String)I` — start index of a named capture group.
pub(crate) fn native_matcher_start_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let name = string_value_from_ref(heap, name_ref).unwrap_or_default();
    let start = matcher_group_bounds_java(heap, m_ref, MatcherGroup::Name(&name))?
        .map_or(-1, |(start, _)| i32::try_from(start).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(start)))
}
/// Native: `Matcher.end()I` — exclusive end index of last match.
pub(crate) fn native_matcher_end(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let end = matcher_group_bounds_java(heap, m_ref, MatcherGroup::Index(0))?
        .map_or(-1, |(_, end)| i32::try_from(end).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(end)))
}
/// Native: `Matcher.end(String)I` — exclusive end index of a named capture group.
pub(crate) fn native_matcher_end_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let name = string_value_from_ref(heap, name_ref).unwrap_or_default();
    let end = matcher_group_bounds_java(heap, m_ref, MatcherGroup::Name(&name))?
        .map_or(-1, |(_, end)| i32::try_from(end).unwrap_or(i32::MAX));
    Ok(Some(Slot::Int(end)))
}
/// Native: `Matcher.reset()Matcher` — reset state against the current input.
pub(crate) fn native_matcher_reset(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    reset_matcher_fields(heap, m_ref)?;
    Ok(Some(Slot::Reference(Some(m_ref))))
}
/// Native: `Matcher.reset(CharSequence)Matcher` — reset state against new input.
pub(crate) fn native_matcher_reset_input(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let input_slot = extract_slot_arg(args, 1);
    heap.get_mut(m_ref)?.fields[MATCHER_INPUT_FIELD] = input_slot;
    reset_matcher_fields(heap, m_ref)?;
    Ok(Some(Slot::Reference(Some(m_ref))))
}
/// Native: `Matcher.appendReplacement(StringBuilder,String)Matcher`.
pub(crate) fn native_matcher_append_replacement_sb(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let builder_ref = extract_ref_arg(args, 1)?;
    let replacement_ref = extract_ref_arg(args, 2)?;
    let replacement = string_value_from_ref(heap, replacement_ref).unwrap_or_default();
    let (match_start, match_end) = last_match_bounds(heap, m_ref)?;
    let append_pos = usize::try_from(matcher_field_int(
        heap,
        m_ref,
        MATCHER_APPEND_POS_FIELD,
        0,
    )?)
    .unwrap_or(0);
    let Some((pattern_str, flags, input)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Err(regex_illegal_state("No match available"));
    };
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let Some(caps) = re.captures_at(&input, match_start) else {
        return Err(regex_illegal_state("No match available"));
    };
    let Some(whole) = caps.get(0) else {
        return Err(regex_illegal_state("No match available"));
    };
    if whole.start() != match_start || whole.end() != match_end {
        return Err(regex_illegal_state("No match available"));
    }
    let mut expanded = String::new();
    caps.expand(&replacement, &mut expanded);
    let safe_append_pos = append_pos.min(match_start);
    append_to_string_builder(heap, builder_ref, &input[safe_append_pos..match_start])?;
    append_to_string_builder(heap, builder_ref, &expanded)?;
    heap.get_mut(m_ref)?.fields[MATCHER_APPEND_POS_FIELD] =
        Slot::Int(i32::try_from(match_end).unwrap_or(i32::MAX));
    Ok(Some(Slot::Reference(Some(m_ref))))
}
/// Native: `Matcher.appendTail(StringBuilder)StringBuilder`.
pub(crate) fn native_matcher_append_tail_sb(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let builder_ref = extract_ref_arg(args, 1)?;
    let append_pos = usize::try_from(matcher_field_int(
        heap,
        m_ref,
        MATCHER_APPEND_POS_FIELD,
        0,
    )?)
    .unwrap_or(0);
    let Some((_, _, input)) = matcher_pattern_input_text(heap, m_ref)? else {
        return Ok(Some(Slot::Reference(Some(builder_ref))));
    };
    let safe_append_pos = append_pos.min(input.len());
    append_to_string_builder(heap, builder_ref, &input[safe_append_pos..])?;
    heap.get_mut(m_ref)?.fields[MATCHER_APPEND_POS_FIELD] =
        Slot::Int(i32::try_from(input.len()).unwrap_or(i32::MAX));
    Ok(Some(Slot::Reference(Some(builder_ref))))
}
/// Native: `Matcher.replaceAll(String)String` — replace all matches.
pub(crate) fn native_matcher_replace_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let repl_ref = extract_ref_arg(args, 1)?;
    let (pat_ref, input_ref) = {
        let obj = heap.get(m_ref)?;
        let Some(Slot::Reference(Some(pat_ref))) = obj.fields.get(MATCHER_PATTERN_FIELD).copied() else {
            return Ok(Some(Slot::Reference(None)));
        };
        let Some(Slot::Reference(Some(input_ref))) = obj.fields.get(MATCHER_INPUT_FIELD).copied() else {
            return Ok(Some(Slot::Reference(None)));
        };
        (pat_ref, input_ref)
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = charsequence_chars(heap, input_ref)?.unwrap_or_default();
    let repl = string_value_from_ref(heap, repl_ref).unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let result = re.replace_all(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}
/// Native: `Matcher.replaceFirst(String)String` — replace first match.
pub(crate) fn native_matcher_replace_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let repl_ref = extract_ref_arg(args, 1)?;
    let (pat_ref, input_ref) = {
        let obj = heap.get(m_ref)?;
        let Some(Slot::Reference(Some(pat_ref))) = obj.fields.get(MATCHER_PATTERN_FIELD).copied() else {
            return Ok(Some(Slot::Reference(None)));
        };
        let Some(Slot::Reference(Some(input_ref))) = obj.fields.get(MATCHER_INPUT_FIELD).copied() else {
            return Ok(Some(Slot::Reference(None)));
        };
        (pat_ref, input_ref)
    };
    let (pattern_str, flags) = pattern_text_and_flags(heap, pat_ref)?;
    let input = charsequence_chars(heap, input_ref)?.unwrap_or_default();
    let repl = string_value_from_ref(heap, repl_ref).unwrap_or_default();
    let re = compile_java_regex_with_flags(&pattern_str, flags)?;
    let result = re.replace(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}