/// Native: `AndThenConsumer.accept(O)V` — runs first then second consumer.
pub(crate) fn native_and_then_consumer_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let arg = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let first = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let second = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    invoke_consumer_accept(first, arg, heap, out, ops)?;
    invoke_consumer_accept(second, arg, heap, out, ops)?;
    Ok(None)
}

/// Native: `Consumer.andThen(Consumer)Consumer` — chains two consumers sequentially.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_consumer_and_then(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let first = args.first().copied().unwrap_or(Slot::Reference(None));
    let second = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/AndThenConsumer".to_string(), 2);
    heap.get_mut(r)?.fields[0] = first;
    heap.get_mut(r)?.fields[1] = second;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `NegatedPredicate.test(O)Z` — inverts the wrapped predicate.
pub(crate) fn native_negated_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let original = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let result = invoke_predicate_test(original, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(!result))))
}

/// Native: `Predicate.negate()Predicate` — logical NOT of a predicate.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_negate(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let original = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/NegatedPredicate".to_string(), 1);
    heap.get_mut(r)?.fields[0] = original;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `OrPredicate.test(O)Z` — either predicate returning true is sufficient.
pub(crate) fn native_or_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let left = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let right = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if la {
        return Ok(Some(Slot::Int(1)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(rb))))
}

/// Native: `Predicate.or(Predicate)Predicate` — logical OR of two predicates.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_or(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let left = args.first().copied().unwrap_or(Slot::Reference(None));
    let right = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/OrPredicate".to_string(), 2);
    heap.get_mut(r)?.fields[0] = left;
    heap.get_mut(r)?.fields[1] = right;
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `AndPredicate.test(O)Z` — both predicates must return true.
pub(crate) fn native_and_predicate_test(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let left = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let right = heap
        .get(this_ref)?
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let la = invoke_predicate_test(left, elem, heap, out, ops)?;
    if !la {
        return Ok(Some(Slot::Int(0)));
    }
    let rb = invoke_predicate_test(right, elem, heap, out, ops)?;
    Ok(Some(Slot::Int(i32::from(rb))))
}

/// Native: `Predicate.and(Predicate)Predicate` — logical AND of two predicates.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_predicate_and(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let left = args.first().copied().unwrap_or(Slot::Reference(None));
    let right = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/AndPredicate".to_string(), 2);
    heap.get_mut(r)?.fields[0] = left;
    heap.get_mut(r)?.fields[1] = right;
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_process_destroy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    heap.destroy_host_process(process_id)?;
    Ok(None)
}

pub(crate) fn native_process_exit_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    let Some(exit_code) = heap.try_host_process_exit_value(process_id)? else {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalThreadStateException".into(),
        });
    };
    Ok(Some(Slot::Int(exit_code)))
}

pub(crate) fn native_process_wait_for(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    Ok(Some(Slot::Int(heap.wait_host_process(process_id)?)))
}

pub(crate) fn native_runtime_exec_array_dir(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(_))) => {}
        _ => return Err(VmError::NullPointerException),
    }
    let command = string_array_from_slot(extract_slot_arg(args, 1), heap)?;
    let cwd = optional_file_path_from_slot(extract_slot_arg(args, 3), heap)?;
    spawn_process_impl(heap, &command, cwd.as_deref())
}

pub(crate) fn native_runtime_exec_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(_))) => {}
        _ => return Err(VmError::NullPointerException),
    }
    let command = string_array_from_slot(extract_slot_arg(args, 1), heap)?;
    spawn_process_impl(heap, &command, None)
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_runtime_get_runtime(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let runtime_ref = heap.allocate("java/lang/Runtime".to_string(), 0);
    Ok(Some(Slot::Reference(Some(runtime_ref))))
}

pub(crate) fn native_process_builder_start(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let builder_obj = heap.get(this_ref)?;
    let command_slot = builder_obj
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let directory_slot = builder_obj
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let command = string_array_from_slot(command_slot, heap)?;
    let cwd = optional_file_path_from_slot(directory_slot, heap)?;
    spawn_process_impl(heap, &command, cwd.as_deref())
}

pub(crate) fn native_process_builder_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let directory_slot = extract_slot_arg(args, 1);
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    builder_obj.fields[1] = directory_slot;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

pub(crate) fn native_process_builder_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let command_slot = extract_slot_arg(args, 1);
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    builder_obj.fields[0] = command_slot;
    builder_obj.fields[1] = Slot::Reference(None);
    Ok(None)
}

/// Native: `Matcher.replaceFirst(String)String` — replace first match.
pub(crate) fn native_matcher_replace_first(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let repl_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.first().copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(1).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Matcher.replaceAll(String)String` — replace all matches.
pub(crate) fn native_matcher_replace_all(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let repl_ref = extract_ref_arg(args, 1)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.first().copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(1).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let repl = heap.get(repl_ref)?.string_value.clone().unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re.replace_all(&input, repl.as_str()).into_owned();
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Matcher.end()I` — exclusive end index of last match.
pub(crate) fn native_matcher_end(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let end = match heap.get(m_ref)?.fields.get(4).copied() {
        Some(Slot::Int(n)) => n,
        _ => 0,
    };
    Ok(Some(Slot::Int(end)))
}

/// Native: `Matcher.start()I` — start index of last match.
pub(crate) fn native_matcher_start(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let start = match heap.get(m_ref)?.fields.get(3).copied() {
        Some(Slot::Int(n)) => n,
        _ => -1,
    };
    Ok(Some(Slot::Int(start)))
}

/// Native: `Matcher.group(int)String` — returns the nth capture group from the last match.
pub(crate) fn native_matcher_group_n(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let n = match args.get(1).copied() {
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    if n == 0 {
        // group(0) == group() — full match
        return native_matcher_group(args, heap, out, control);
    }
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.first().copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(1).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let start = match fields.get(3).copied() {
        Some(Slot::Int(s)) if s >= 0 => usize::try_from(s).unwrap_or(0),
        _ => return Ok(Some(Slot::Reference(None))),
    };
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    if let Some(caps) = re.captures_at(&input, start)
        && let Some(g) = caps.get(n)
    {
        let s = heap.allocate_string(g.as_str().to_string());
        return Ok(Some(Slot::Reference(Some(s))));
    }
    Ok(Some(Slot::Reference(None)))
}

/// Native: `Matcher.group()String` — returns text of last match.
pub(crate) fn native_matcher_group(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let matched = heap.get(m_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(matched);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Matcher.matches()Z` — full-string match (resets position).
pub(crate) fn native_matcher_matches(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.first().copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(1).copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    if result {
        let end = i32::try_from(input.len()).unwrap_or(0);
        heap.get_mut(m_ref)?.fields[3] = Slot::Int(0);
        heap.get_mut(m_ref)?.fields[4] = Slot::Int(end);
        heap.get_mut(m_ref)?.string_value = Some(input);
    }
    Ok(Some(Slot::Int(i32::from(result))))
}

/// Native: `Matcher.find()Z` — finds next match; advances position.
pub(crate) fn native_matcher_find(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let m_ref = extract_ref_arg(args, 0)?;
    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.first().copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let input_slot = fields.get(1).copied().unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(input_ref)) = input_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let pos = match fields.get(2).copied() {
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    if let Some(m) = re.find_at(&input, pos) {
        let start = i32::try_from(m.start()).unwrap_or(0);
        let end = i32::try_from(m.end()).unwrap_or(0);
        let matched = m.as_str().to_string();
        heap.get_mut(m_ref)?.fields[2] = Slot::Int(end); // advance past match
        heap.get_mut(m_ref)?.fields[3] = Slot::Int(start);
        heap.get_mut(m_ref)?.fields[4] = Slot::Int(end);
        heap.get_mut(m_ref)?.string_value = Some(matched);
        Ok(Some(Slot::Int(1)))
    } else {
        heap.get_mut(m_ref)?.fields[3] = Slot::Int(-1);
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `Pattern.matches(String,CharSequence)Z` — static full-string match.
pub(crate) fn native_pattern_matches_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_ref = extract_ref_arg(args, 1)?;
    let pattern_str = heap.get(pat_ref)?.string_value.clone().unwrap_or_default();
    let input = heap
        .get(input_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let re = compile_java_regex(&pattern_str)?;
    let result = re
        .find(&input)
        .is_some_and(|m| m.start() == 0 && m.end() == input.len());
    Ok(Some(Slot::Int(i32::from(result))))
}

/// Native: `Pattern.matcher(CharSequence)Matcher` — creates a Matcher.
pub(crate) fn native_pattern_matcher(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let pat_ref = extract_ref_arg(args, 0)?;
    let input_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    // fields: [0]=pattern_ref, [1]=input_ref, [2]=pos, [3]=match_start, [4]=match_end
    let m_ref = heap.allocate("java/util/regex/Matcher".to_string(), 5);
    heap.get_mut(m_ref)?.fields[0] = Slot::Reference(Some(pat_ref));
    heap.get_mut(m_ref)?.fields[1] = input_slot;
    heap.get_mut(m_ref)?.fields[2] = Slot::Int(0);
    heap.get_mut(m_ref)?.fields[3] = Slot::Int(-1);
    heap.get_mut(m_ref)?.fields[4] = Slot::Int(0);
    Ok(Some(Slot::Reference(Some(m_ref))))
}

/// Native: `Pattern.compile(String)Pattern` — static factory.
pub(crate) fn native_pattern_compile(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let pat_str_ref = extract_ref_arg(args, 0)?;
    let pattern_str = heap
        .get(pat_str_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    // Validate the regex eagerly so we fail here not at match time.
    compile_java_regex(&pattern_str)?;
    let pat_ref = heap.allocate("java/util/regex/Pattern".to_string(), 0);
    heap.get_mut(pat_ref)?.string_value = Some(pattern_str);
    Ok(Some(Slot::Reference(Some(pat_ref))))
}

/// Native: `Collection.toArray(Object[])` — preserves the requested array type.
pub(crate) fn native_collection_to_array_with_seed_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let seed_array_ref = extract_ref_arg(args, 1)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let seed_class_name = heap.get(seed_array_ref)?.class_name.clone();
    let seed_len = heap.get(seed_array_ref)?.fields.len();

    if seed_len < elements.len() {
        let new_array_ref = allocate_reference_array_from_slots(heap, &seed_class_name, &elements)?;
        return Ok(Some(Slot::Reference(Some(new_array_ref))));
    }

    let seed_array = heap.get_mut(seed_array_ref)?;
    for slot in &mut seed_array.fields {
        *slot = Slot::Reference(None);
    }
    for (idx, element) in elements.iter().enumerate() {
        seed_array.fields[idx] = *element;
    }
    Ok(Some(Slot::Reference(Some(seed_array_ref))))
}

/// Native: `Collection.toArray()` — copies Duke-backed collection elements into `Object[]`.
pub(crate) fn native_collection_to_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let elements = collection_elements_from_ref(heap, this_ref)?;
    let array_ref = allocate_reference_array_from_slots(heap, "[Ljava/lang/Object;", &elements)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_short_shortvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_short_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Short".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_short_parseshort_radix(
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
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_short_parseshort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i16::MIN), i32::from(i16::MAX))?;
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_byte_bytevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

pub(crate) fn native_byte_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = extract_int_arg(args, 0)?;
    let r = heap.allocate("java/lang/Byte".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

pub(crate) fn native_byte_parsebyte_radix(
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
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_byte_parsebyte(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val =
        parse_bounded_i32_from_string_arg(args, heap, i32::from(i8::MIN), i32::from(i8::MAX))?;
    Ok(Some(Slot::Int(val)))
}

pub(crate) fn native_reflect_constructor_new_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let constructor_ref = extract_ref_arg(args, 0)?;
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 1))?;
    let constructor = reflected_method_handle(heap, constructor_ref)?;

    if !constructor.is_public && !constructor.is_accessible {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let instance_ref = ops.allocate_instance(heap, output, &constructor.declaring_class_key)?;
    let invoke_args = build_reflection_invoke_args(
        heap,
        Slot::Reference(Some(instance_ref)),
        &constructor.descriptor,
        invoke_arg_slots,
        false,
    )?;

    match ops.invoke(
        heap,
        output,
        &constructor.declaring_class_key,
        &constructor.method_name,
        &constructor.descriptor,
        invoke_args,
    ) {
        Ok(_) => Ok(Some(Slot::Reference(Some(instance_ref)))),
        Err(VmError::JavaException { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_reflect_method_invoke(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let invoke_arg_slots = reflection_array_elements(heap, extract_slot_arg(args, 2))?;
    let method = reflected_method_handle(heap, method_ref)?;

    if !method.is_public && !method.is_accessible {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let invoke_args = build_reflection_invoke_args(
        heap,
        target_slot,
        &method.descriptor,
        invoke_arg_slots,
        method.is_static,
    )?;

    ops.ensure_loaded(&method.declaring_class_key)?;
    match ops.invoke(
        heap,
        output,
        &method.declaring_class_key,
        &method.method_name,
        &method.descriptor,
        invoke_args,
    ) {
        Ok(result) => Ok(Some(box_reflection_return_value(
            heap,
            descriptor_return_type(&method.descriptor),
            result,
        )?)),
        Err(VmError::JavaException { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_reflect_field_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let target_slot = extract_slot_arg(args, 1);
    let field = reflected_field_handle(heap, field_ref)?;

    if !field.is_public && !field.is_accessible {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let field_type = field.descriptor.chars().next().unwrap_or('L');
    let raw_value = if field.is_static {
        ops.ensure_class_initialized(heap, output, &field.declaring_class_key)?;
        ops.read_static_field(&field.declaring_class_key, &field.field_name)?
    } else {
        let Slot::Reference(Some(target_ref)) = target_slot else {
            return Err(VmError::NullPointerException);
        };
        ops.read_instance_field(
            heap,
            target_ref,
            &field.declaring_class_key,
            &field.field_name,
        )?
    };

    Ok(Some(box_reflection_return_value(
        heap,
        field_type,
        Some(raw_value),
    )?))
}

pub(crate) fn native_reflect_field_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_name_slot(heap, field_ref)?))
}

pub(crate) fn native_reflect_executable_get_parameter_types(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    let member = reflected_method_handle(heap, member_ref)?;
    let parameter_descriptors = parse_arg_descriptors(&member.descriptor);
    let mut class_refs = Vec::with_capacity(parameter_descriptors.len());
    for descriptor in parameter_descriptors {
        let Slot::Reference(Some(class_ref)) = descriptor_class_slot_from_source(
            heap,
            ops,
            &descriptor,
            Some(member.declaring_class_key.as_str()),
        )?
        else {
            return Err(VmError::TypeMismatch {
                expected: "class reference",
                got: "other",
            });
        };
        class_refs.push(class_ref);
    }
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/Class;", &class_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_reflect_method_get_parameter_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let method = reflected_method_handle(heap, method_ref)?;
    let count = i32::try_from(parse_arg_count(&method.descriptor)).unwrap_or(i32::MAX);
    Ok(Some(Slot::Int(count)))
}

pub(crate) fn native_reflection_member_get_declaring_class(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let member_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_declaring_class_slot(
        heap, member_ref,
    )?))
}

pub(crate) fn native_reflect_field_get_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let field_ref = extract_ref_arg(args, 0)?;
    let field = reflected_field_handle(heap, field_ref)?;
    Ok(Some(descriptor_class_slot_from_source(
        heap,
        ops,
        &field.descriptor,
        Some(field.declaring_class_key.as_str()),
    )?))
}

pub(crate) fn native_reflect_method_get_return_type(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    let method = reflected_method_handle(heap, method_ref)?;
    Ok(Some(descriptor_class_slot_from_source(
        heap,
        ops,
        method_return_descriptor(&method.descriptor),
        Some(method.declaring_class_key.as_str()),
    )?))
}

pub(crate) fn native_reflect_constructor_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let constructor_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_declaring_class_name_slot(
        heap,
        constructor_ref,
    )?))
}

pub(crate) fn native_reflect_method_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let method_ref = extract_ref_arg(args, 0)?;
    Ok(Some(reflection_member_name_slot(heap, method_ref)?))
}

pub(crate) fn native_class_get_fields(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_refs = collect_public_reflected_fields(ops, &class_key)?
        .into_iter()
        .map(|(declaring_class, field)| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Field",
                &declaring_class,
                &field.name,
                &field.descriptor,
                field.is_public,
                field.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Field;", &field_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_declared_fields(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let field_refs = reflected
        .fields
        .into_iter()
        .map(|field| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Field",
                &class_key,
                &field.name,
                &field.descriptor,
                field.is_public,
                field.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Field;", &field_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_constructors(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructor_refs = reflected_constructors(reflected, true)
        .into_iter()
        .map(|constructor| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Constructor",
                &class_key,
                &constructor.name,
                &constructor.descriptor,
                constructor.is_public,
                constructor.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/reflect/Constructor;", &constructor_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_methods(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_refs = collect_public_reflected_methods(ops, &class_key)?
        .into_iter()
        .map(|(declaring_class, method)| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Method",
                &declaring_class,
                &method.name,
                &method.descriptor,
                method.is_public,
                method.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Method;", &method_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_declared_constructors(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructor_refs = reflected_constructors(reflected, false)
        .into_iter()
        .map(|constructor| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Constructor",
                &class_key,
                &constructor.name,
                &constructor.descriptor,
                constructor.is_public,
                constructor.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref =
        allocate_reference_array(heap, "[Ljava/lang/reflect/Constructor;", &constructor_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_get_declared_methods(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let method_refs = reflected
        .methods
        .into_iter()
        .filter(|method| method.name != "<init>" && method.name != "<clinit>")
        .map(|method| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Method",
                &class_key,
                &method.name,
                &method.descriptor,
                method.is_public,
                method.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Method;", &method_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

pub(crate) fn native_class_new_instance(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let constructors = reflected_constructors(reflected, false);
    let Some(constructor) = constructors
        .iter()
        .find(|constructor| constructor.descriptor == "()V")
        .cloned()
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/InstantiationException".to_string(),
        });
    };
    if !constructor.is_public {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let instance_ref = ops.allocate_instance(heap, output, &class_key)?;
    match ops.invoke(
        heap,
        output,
        &class_key,
        "<init>",
        &constructor.descriptor,
        vec![Slot::Reference(Some(instance_ref))],
    ) {
        Ok(_) => Ok(Some(Slot::Reference(Some(instance_ref)))),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_get_constructor(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 1))?;

    let Some(constructor) = lookup_reflected_constructor(reflected, &parameter_descriptor, true)
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let constructor_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Constructor",
        &class_key,
        &constructor.name,
        &constructor.descriptor,
        constructor.is_public,
        constructor.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(constructor_ref))))
}

pub(crate) fn native_class_get_field(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_name = string_value_from_ref(heap, name_ref)?;

    let Some((declaring_class, field)) =
        lookup_public_reflected_field(ops, &class_key, &field_name)?
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchFieldException".to_string(),
        });
    };

    let field_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Field",
        &declaring_class,
        &field.name,
        &field.descriptor,
        field.is_public,
        field.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(field_ref))))
}

pub(crate) fn native_class_get_declared_constructor(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 1))?;

    let Some(constructor) = lookup_reflected_constructor(reflected, &parameter_descriptor, false)
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let constructor_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Constructor",
        &class_key,
        &constructor.name,
        &constructor.descriptor,
        constructor.is_public,
        constructor.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(constructor_ref))))
}

pub(crate) fn native_class_get_declared_field(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let field_name = string_value_from_ref(heap, name_ref)?;
    let reflected = ops.inspect_class(&class_key)?;

    let Some(field) = reflected
        .fields
        .into_iter()
        .find(|field| field.name == field_name)
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchFieldException".to_string(),
        });
    };

    let field_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Field",
        &class_key,
        &field.name,
        &field.descriptor,
        field.is_public,
        field.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(field_ref))))
}

pub(crate) fn native_class_get_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_name = string_value_from_ref(heap, name_ref)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 2))?;

    let Some((declaring_class, method)) =
        lookup_public_reflected_method(ops, &class_key, &method_name, &parameter_descriptor)?
    else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let method_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Method",
        &declaring_class,
        &method.name,
        &method.descriptor,
        method.is_public,
        method.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(method_ref))))
}

pub(crate) fn native_class_get_declared_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let method_name = string_value_from_ref(heap, name_ref)?;
    let reflected = ops.inspect_class(&class_key)?;
    let parameter_descriptor =
        parameter_descriptor_from_class_array(heap, extract_slot_arg(args, 2))?;

    let Some(method) = reflected.methods.into_iter().find(|method| {
        method.name == method_name
            && descriptor_parameter_part(&method.descriptor) == parameter_descriptor
    }) else {
        return Err(VmError::JavaException {
            class_name: "java/lang/NoSuchMethodException".to_string(),
        });
    };

    let method_ref = allocate_reflection_member_object(
        heap,
        "java/lang/reflect/Method",
        &class_key,
        &method.name,
        &method.descriptor,
        method.is_public,
        method.is_static,
    )?;
    Ok(Some(Slot::Reference(Some(method_ref))))
}

pub(crate) fn native_boot_launched_class_loader_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let exploded = extract_int_arg(args, 1)?;
    let archive_ref = extract_ref_arg(args, 2)?;
    let _ = extract_ref_arg(args, 3)?;
    match args.get(4) {
        Some(Slot::Reference(_)) => {
            let exploded_slot = ops.instance_field_slot(
                "org/springframework/boot/loader/launch/LaunchedClassLoader",
                "exploded",
            )?;
            let root_archive_slot = ops.instance_field_slot(
                "org/springframework/boot/loader/launch/LaunchedClassLoader",
                "rootArchive",
            )?;
            let loader_obj = heap.get_mut(this_ref)?;
            loader_obj.fields[exploded_slot] = Slot::Int(i32::from(exploded != 0));
            loader_obj.fields[root_archive_slot] = Slot::Reference(Some(archive_ref));
            Ok(None)
        }
        _ => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

pub(crate) fn native_boot_exploded_archive_get_class_path_urls(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let archive_ref = extract_ref_arg(args, 0)?;
    let include_predicate_ref = extract_ref_arg(args, 1)?;
    let search_predicate_ref = extract_ref_arg(args, 2)?;
    let root_directory_ref = archive_file_ref_at(heap, archive_ref, 0)?;
    let root_path = file_path_from_ref(root_directory_ref, heap)?;

    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    let mut pending: VecDeque<std::path::PathBuf> = list_directory_children_sorted(&root_path)?
        .into_iter()
        .collect();
    while let Some(entry_path) = pending.pop_front() {
        let is_directory = entry_path.is_dir();
        let entry_name =
            exploded_archive_relative_entry_name(&root_path, &entry_path, is_directory);
        if is_directory
            && boot_archive_predicate_accepts(
                search_predicate_ref,
                &entry_name,
                true,
                heap,
                out,
                ops,
            )?
        {
            let children = list_directory_children_sorted(&entry_path)?;
            for child in children.into_iter().rev() {
                pending.push_front(child);
            }
        }

        if boot_archive_predicate_accepts(
            include_predicate_ref,
            &entry_name,
            is_directory,
            heap,
            out,
            ops,
        )? {
            boot_archive_hashset_add_url(
                set_ref,
                path_to_file_url(&entry_path),
                heap,
                out,
                control,
            )?;
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

pub(crate) fn native_boot_jar_file_archive_get_class_path_urls(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let archive_ref = extract_ref_arg(args, 0)?;
    let include_predicate_ref = extract_ref_arg(args, 1)?;
    let _ = extract_ref_arg(args, 2)?;
    let archive_file_ref = archive_file_ref_at(heap, archive_ref, 0)?;
    let archive_path = file_path_from_ref(archive_file_ref, heap)?;
    let reader = open_boot_archive_reader(&archive_path)?;
    let mut entry_names: Vec<String> = reader.entry_names().map(ToOwned::to_owned).collect();
    entry_names.sort_unstable();

    let set_ref = heap.allocate("java/util/HashSet".to_string(), 1);
    native_hashset_init(&[Slot::Reference(Some(set_ref))], heap, out, control)?;

    for entry_name in entry_names {
        let is_directory = entry_name.ends_with('/');
        if boot_archive_predicate_accepts(
            include_predicate_ref,
            &entry_name,
            is_directory,
            heap,
            out,
            ops,
        )? {
            let url_spec = format!("jar:{}!/{}", path_to_file_url(&archive_path), entry_name);
            boot_archive_hashset_add_url(set_ref, url_spec, heap, out, control)?;
        }
    }

    Ok(Some(Slot::Reference(Some(set_ref))))
}

pub(crate) fn native_boot_archive_entry_is_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let entry_ref = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(i32::from(
        boot_archive_entry_is_directory_flag(heap, entry_ref)?,
    ))))
}

pub(crate) fn native_boot_archive_entry_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let entry_ref = extract_ref_arg(args, 0)?;
    let Some(slot @ Slot::Reference(Some(_))) = heap
        .get(entry_ref)?
        .fields
        .get(BOOT_ARCHIVE_ENTRY_NAME_SLOT)
        .copied()
    else {
        return Err(VmError::NullPointerException);
    };
    Ok(Some(slot))
}

pub(crate) fn native_attributes_get_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let attributes_ref = extract_ref_arg(args, 0)?;
    let key_ref = extract_ref_arg(args, 1)?;
    let Some(Slot::Reference(Some(raw_ref))) = heap.get(attributes_ref)?.fields.first().copied()
    else {
        return Err(VmError::NullPointerException);
    };
    let manifest_text = string_value_from_ref(heap, raw_ref)?;
    let key = string_value_from_ref(heap, key_ref)?;
    let result = manifest_attribute_value(manifest_text.as_bytes(), &key)
        .map_or(Slot::Reference(None), |value| {
            Slot::Reference(Some(heap.allocate_string(value)))
        });
    Ok(Some(result))
}

pub(crate) fn native_manifest_get_main_attributes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let manifest_ref = extract_ref_arg(args, 0)?;
    let Some(Slot::Reference(Some(raw_ref))) = heap.get(manifest_ref)?.fields.first().copied()
    else {
        return Err(VmError::NullPointerException);
    };
    let attributes_ref = heap.allocate("java/util/jar/Attributes".to_string(), 1);
    heap.get_mut(attributes_ref)?.fields[0] = Slot::Reference(Some(raw_ref));
    Ok(Some(Slot::Reference(Some(attributes_ref))))
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
pub(crate) fn native_posix_file_permissions_as_file_attribute(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let attribute_ref = heap.allocate("java/nio/file/attribute/FileAttribute".to_string(), 0);
    Ok(Some(Slot::Reference(Some(attribute_ref))))
}

pub(crate) fn native_paths_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let first_ref = extract_ref_arg(args, 0)?;
    let mut path = std::path::PathBuf::from(string_value_from_ref(heap, first_ref)?);
    let more_slot = extract_slot_arg(args, 1);
    match more_slot {
        Slot::Reference(Some(array_ref)) => {
            let segments = heap.get(array_ref)?.fields.clone();
            for segment in segments {
                let Slot::Reference(Some(segment_ref)) = segment else {
                    return Err(VmError::NullPointerException);
                };
                path.push(string_value_from_ref(heap, segment_ref)?);
            }
        }
        Slot::Reference(None) => {}
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    }
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().into_owned(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}

pub(crate) fn native_path_to_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_slot = heap
        .get(this_ref)?
        .fields
        .first()
        .copied()
        .ok_or(VmError::NullPointerException)?;
    let file_ref = heap.allocate("java/io/File".to_string(), 1);
    heap.get_mut(file_ref)?.fields[0] = path_slot;
    Ok(Some(Slot::Reference(Some(file_ref))))
}

pub(crate) fn native_path_of(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let uri_ref = extract_ref_arg(args, 0)?;
    let uri = string_backed_object_value(heap, uri_ref)?;
    let path = file_url_to_path(&uri)?;
    let path_ref = allocate_string_backed_object(
        heap,
        "java/nio/file/Path",
        path.to_string_lossy().to_string(),
    )?;
    Ok(Some(Slot::Reference(Some(path_ref))))
}

pub(crate) fn native_url_to_uri(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let spec = string_backed_object_value(heap, this_ref)?;
    let uri_ref = allocate_string_backed_object(heap, "java/net/URI", spec)?;
    Ok(Some(Slot::Reference(Some(uri_ref))))
}

pub(crate) fn native_code_source_get_location(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(VmError::NullPointerException),
    }
}

pub(crate) fn native_protection_domain_get_code_source(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Err(VmError::NullPointerException),
    }
}

pub(crate) fn native_class_get_protection_domain(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    let pd_ref = heap.allocate("java/security/ProtectionDomain".to_string(), 1);
    let code_source_slot = if let Some(path) = ops.code_source_for_class(&class_key)? {
        let url_ref = allocate_string_backed_object(
            heap,
            "java/net/URL",
            path_to_file_url(std::path::Path::new(&path)),
        )?;
        let code_source_ref = heap.allocate("java/security/CodeSource".to_string(), 1);
        heap.get_mut(code_source_ref)?.fields[0] = Slot::Reference(Some(url_ref));
        Slot::Reference(Some(code_source_ref))
    } else {
        Slot::Reference(None)
    };
    heap.get_mut(pd_ref)?.fields[0] = code_source_slot;
    Ok(Some(Slot::Reference(Some(pd_ref))))
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_class_loader_register_as_parallel_capable(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(Slot::Int(1)))
}

pub(crate) fn native_class_get_class_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let class_key = class_key_from_ref(heap, class_ref)?;
    Ok(Some(Slot::Reference(
        ops.runtime_loader_for_class(&class_key)?,
    )))
}

pub(crate) fn native_class_for_name_with_loader(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let binary_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    let (load_result, class_key) = match args.get(2) {
        Some(Slot::Reference(Some(loader_ref))) => (
            ops.ensure_loaded_with_runtime_loader(heap, *loader_ref, &internal_name),
            ops.class_key_for_runtime_loader(heap, *loader_ref, &internal_name)?,
        ),
        Some(Slot::Reference(None)) | None => (
            ops.ensure_loaded(&internal_name),
            ops.class_key_for_loaded_class(&internal_name)?,
        ),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    match load_result {
        Ok(()) => {
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(VmError::ClassNotFound { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

pub(crate) fn native_class_for_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let name_ref = extract_ref_arg(args, 0)?;
    let binary_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    match ops.ensure_loaded(&internal_name) {
        Ok(()) => {
            let class_key = ops.class_key_for_loaded_class(&internal_name)?;
            let class_ref = allocate_class_object(heap, &class_key)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(VmError::ClassNotFound { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_void_noop(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `Class.desiredAssertionStatus()` - Duke currently runs with assertions disabled.
pub(crate) fn native_class_desired_assertion_status(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let _ = extract_ref_arg(args, 0)?;
    Ok(Some(Slot::Int(0)))
}

pub(crate) fn native_class_get_package_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let package_name = if internal_name.starts_with('[') {
        String::new()
    } else {
        internal_name
            .rsplit_once('/')
            .map_or_else(String::new, |(package, _)| package.replace('/', "."))
    };
    let package_ref = heap.allocate_string(package_name);
    Ok(Some(Slot::Reference(Some(package_ref))))
}

pub(crate) fn native_class_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let name_ref = heap.allocate_string(internal_name_to_binary_name(&internal_name));
    Ok(Some(Slot::Reference(Some(name_ref))))
}

/// Native: `Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`
/// Searches heap for enum constants of the given class matching the name.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn native_enum_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let class_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let target_name = heap.get(name_ref)?.string_value.clone().unwrap_or_default();
    let enum_class_name = heap
        .get(class_ref)?
        .string_value
        .clone()
        .unwrap_or_default();

    let obj_count = heap.len();
    for i in 0..obj_count {
        let obj = heap.get(i as u64)?;
        if obj.class_name == enum_class_name
            && obj.fields.len() >= 2
            && let Some(Slot::Reference(Some(name_r))) = obj.fields.first()
            && let Ok(name_obj) = heap.get(*name_r)
            && name_obj.string_value.as_deref() == Some(target_name.as_str())
        {
            return Ok(Some(Slot::Reference(Some(i as u64))));
        }
    }

    Err(VmError::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    })
}

/// Native: `Enum.name()Ljava/lang/String;`
pub(crate) fn native_enum_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Enum.ordinal()I`
pub(crate) fn native_enum_ordinal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let obj = heap.get(this_ref)?;
    match obj.fields.get(1) {
        Some(Slot::Int(v)) => Ok(Some(Slot::Int(*v))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Enum.<init>(Ljava/lang/String;I)V` — stores name + ordinal.
/// args: `[this_ref, name_ref, ordinal_int]`
pub(crate) fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_slot = extract_slot_arg(args, 1);
    let ordinal = match args.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() >= 2 {
        obj.fields[0] = name_slot;
        obj.fields[1] = Slot::Int(ordinal);
    }
    Ok(None)
}

/// Native: `Stack.size()I`
pub(crate) fn native_stack_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_size(args, heap, out, control)
}

/// Native: `Stack.empty()Z` — returns true if the stack is empty.
pub(crate) fn native_stack_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_is_empty(args, heap, out, control)
}

/// Native: `Stack.peek()E` — returns the top element without removal.
pub(crate) fn native_stack_peek(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_linked_list_peek_last(args, heap, out, control)
}

/// Native: `Stack.pop()E` — removes and returns the top element.
pub(crate) fn native_stack_pop(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_linked_list_remove_last(args, heap, out, control)
}

/// Native: `Stack.push(E)E` — appends to tail, returns the element.
pub(crate) fn native_stack_push(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let elem = args.get(1).copied().unwrap_or(Slot::Reference(None));
    native_arraylist_add(args, heap, out, control)?;
    Ok(Some(elem))
}

/// Native: `Stack.<init>()V`
pub(crate) fn native_stack_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_arraylist_init(args, heap, out, control)
}

/// Native: `Throwable.getMessage()String` — returns the stored detail message.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_throwable_get_message(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let msg = heap.get(*r)?.string_value.clone();
            let slot = msg.map_or(Slot::Reference(None), |s| {
                Slot::Reference(Some(heap.allocate_string(s)))
            });
            Ok(Some(slot))
        }
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Throwable.getCause()Throwable` — returns the stored cause.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_throwable_get_cause(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let cause = heap
                .get(*r)?
                .fields
                .first()
                .copied()
                .unwrap_or(Slot::Reference(None));
            Ok(Some(cause))
        }
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// `Throwable.addSuppressed(Throwable suppressed)V`
///
/// No-op stub. Control flow is handled entirely by the bytecode desugaring —
/// `addSuppressed` only affects what `getSuppressed()` returns, which is not
/// yet implemented. Suppressed exception is silently dropped.
///
/// Signature: `args[0]` = this (Throwable), `args[1]` = suppressed (Throwable)
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn native_throwable_add_suppressed(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _stdout: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `ZipEntry.getMethod() -> int`
pub(crate) fn native_zip_entry_get_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(heap.get(this_ref)?.fields[5]))
}

/// Native: `ZipEntry.getSize() -> long`
pub(crate) fn native_zip_entry_get_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[3] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[4] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}

/// Native: `ZipEntry.getCompressedSize() -> long`
pub(crate) fn native_zip_entry_get_compressed_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[1] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[2] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}

/// Native: `ZipEntry.getName() -> String`
pub(crate) fn native_zip_entry_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    Ok(Some(heap.get(this_ref)?.fields[0]))
}

/// Native: `ZipFile.size() -> int`
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_zip_file_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let count = heap.zip_entry_count(fd)?;
    Ok(Some(Slot::Int(count as i32)))
}

/// Native: `ZipFile.close()`
pub(crate) fn native_zip_file_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => return Ok(None),
    };
    heap.close_host_file(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ZipFile.getEntry(String) -> ZipEntry` — look up an entry by name.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub(crate) fn native_zip_file_get_entry(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let name_ref = extract_ref_arg(args, 1)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let entry_name = heap
        .get(name_ref)?
        .string_value
        .as_deref()
        .ok_or(VmError::NullPointerException)?
        .to_string();
    let info = heap.zip_get_entry_info(fd, &entry_name)?;
    let Some(info) = info else {
        return Ok(Some(Slot::Reference(None)));
    };
    // Allocate a ZipEntry HeapObject with 6 fields.
    let name_heap_ref = heap.allocate_string(info.name);
    let entry_ref = heap.allocate("java/util/zip/ZipEntry".to_string(), 6);
    let entry_obj = heap.get_mut(entry_ref)?;
    entry_obj.fields[0] = Slot::Reference(Some(name_heap_ref));
    entry_obj.fields[1] = Slot::Int(info.compressed_size as i32);
    entry_obj.fields[2] = Slot::Int((info.compressed_size >> 32) as i32);
    entry_obj.fields[3] = Slot::Int(info.uncompressed_size as i32);
    entry_obj.fields[4] = Slot::Int((info.uncompressed_size >> 32) as i32);
    entry_obj.fields[5] = Slot::Int(i32::from(info.compression_method));
    Ok(Some(Slot::Reference(Some(entry_ref))))
}

pub(crate) fn native_boot_exploded_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    if let Some(slot @ Slot::Reference(Some(_))) = heap
        .get(this_ref)?
        .fields
        .get(BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT)
        .copied()
    {
        return Ok(Some(slot));
    }

    let root_directory_ref = match heap
        .get(this_ref)?
        .fields
        .get(BOOT_EXPLODED_ARCHIVE_ROOT_DIRECTORY_SLOT)
        .copied()
    {
        Some(Slot::Reference(Some(root_directory_ref))) => root_directory_ref,
        Some(Slot::Reference(None)) | None => return Err(VmError::NullPointerException),
        Some(_) => {
            return Err(VmError::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    let manifest_path = file_path_from_ref(root_directory_ref, heap)?.join("META-INF/MANIFEST.MF");
    let manifest_bytes = match std::fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(Slot::Reference(None)));
        }
        Err(_) => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        }
    };
    let manifest_ref = allocate_manifest_from_bytes(heap, &manifest_bytes)?;
    heap.get_mut(this_ref)?.fields[BOOT_EXPLODED_ARCHIVE_MANIFEST_SLOT] =
        Slot::Reference(Some(manifest_ref));
    Ok(Some(Slot::Reference(Some(manifest_ref))))
}

pub(crate) fn native_boot_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.class_name.as_str() {
        "org/springframework/boot/loader/launch/JarFileArchive" => {
            native_boot_jar_file_archive_get_manifest(args, heap, out, control)
        }
        "org/springframework/boot/loader/launch/ExplodedArchive" => {
            native_boot_exploded_archive_get_manifest(args, heap, out, control)
        }
        _ => Err(VmError::MethodNotFound {
            name: format!("{}.getManifest", heap.get(this_ref)?.class_name),
            descriptor: "()Ljava/util/jar/Manifest;".to_string(),
        }),
    }
}

pub(crate) fn native_boot_jar_file_archive_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let jar_file_ref = match heap
        .get(this_ref)?
        .fields
        .get(BOOT_JAR_FILE_ARCHIVE_JAR_FILE_SLOT)
        .copied()
    {
        Some(Slot::Reference(Some(jar_file_ref))) => jar_file_ref,
        Some(Slot::Reference(None)) | None => return Err(VmError::NullPointerException),
        Some(_) => {
            return Err(VmError::TypeMismatch {
                expected: "reference",
                got: "other",
            });
        }
    };
    native_jar_file_get_manifest(&[Slot::Reference(Some(jar_file_ref))], heap, out, control)
}

pub(crate) fn native_boot_nested_jar_file_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_jar_file_get_manifest(args, heap, out, control)
}

pub(crate) fn native_jar_file_get_manifest(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = extract_io_fd(heap, this_ref)?;
    let Ok(manifest_bytes) = heap.zip_read_entry(fd, "META-INF/MANIFEST.MF") else {
        return Ok(Some(Slot::Reference(None)));
    };
    let manifest_ref = allocate_manifest_from_bytes(heap, &manifest_bytes)?;
    Ok(Some(Slot::Reference(Some(manifest_ref))))
}

pub(crate) fn native_jar_file_init_with_mode_and_version(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let forwarded_args = match args {
        [this_slot, file_slot, ..] => [*this_slot, *file_slot],
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "this,file",
                got: "other",
            });
        }
    };
    native_jar_file_init_from_file(&forwarded_args, heap, out, control)
}

/// Native: `JarFile.<init>(File)` — open and index a JAR archive from a File object.
pub(crate) fn native_jar_file_init_from_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let file_ref = extract_ref_arg(args, 1)?;
    let path = file_path_from_ref(file_ref, heap)?;
    let fd = heap.open_host_zip(&path)?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(fd);
    Ok(None)
}

/// Native: `ZipFile.<init>(String)` — open and index a ZIP/JAR archive.
pub(crate) fn native_zip_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_ref = extract_ref_arg(args, 1)?;
    let path_str = heap
        .get(path_ref)?
        .string_value
        .as_deref()
        .ok_or(VmError::NullPointerException)?
        .to_string();
    let fd = heap.open_host_zip(std::path::Path::new(&path_str))?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(fd);
    Ok(None)
}

/// Native: `Socket.close()` — closes both OS handles (fdRead and fdWrite).
pub(crate) fn native_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd_read = extract_io_fd(heap, this_ref)?;
    let fd_write = extract_io_fd_at(heap, this_ref, 1)?;
    heap.close_host_file(fd_read);
    heap.close_host_file(fd_write);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0);
    Ok(None)
}

/// Native: `Socket.<init>(String host, int port)` — connects to host:port.
pub(crate) fn native_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let host_ref = extract_ref_arg(args, 1)?;
    let host = heap
        .get(host_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    let port = extract_int_arg(args, 2)?;
    let addr = format!("{host}:{port}");
    let (reader_id, writer_id) = heap.connect_socket(&addr)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    obj.fields[0] = Slot::Int(reader_id);
    obj.fields[1] = Slot::Int(writer_id);
    Ok(None)
}

/// Native: `ServerSocket.close()` — closes the OS listener and zeros the fd field.
pub(crate) fn native_server_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        Some(Slot::Int(_)) => return Ok(None), // already closed — idempotent
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    heap.close_host_file(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0); // also zero cached port so getLocalPort() returns 0 after close
    Ok(None)
}

/// Native: `ServerSocket.getLocalPort()` — returns the bound port.
pub(crate) fn native_server_socket_get_local_port(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(port)) => Ok(Some(Slot::Int(*port))),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

/// Native: `ServerSocket.accept()` — blocks until a client connects, returns a Socket.
pub(crate) fn native_server_socket_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let server_fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let (reader_id, writer_id) = heap.accept_connection(server_fd)?;
    // Allocate a new Socket object with fdRead=reader_id, fdWrite=writer_id
    let socket_ref = heap.allocate("java/net/Socket".to_string(), 2);
    heap.get_mut(socket_ref)?.fields[0] = Slot::Int(reader_id);
    heap.get_mut(socket_ref)?.fields[1] = Slot::Int(writer_id);
    Ok(Some(Slot::Reference(Some(socket_ref))))
}

/// Native: `ServerSocket.<init>(int port)` — binds to 0.0.0.0:{port}.
pub(crate) fn native_server_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let port = extract_int_arg(args, 1)?;
    let addr = format!("0.0.0.0:{port}");
    let server_id = heap.bind_server_socket(&addr)?;
    let actual_port = heap.server_socket_local_port(server_id)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    obj.fields[0] = Slot::Int(server_id);
    obj.fields[1] = Slot::Int(actual_port);
    Ok(None)
}

pub(crate) fn native_file_is_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.is_dir()))))
}

pub(crate) fn native_file_is_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.is_file()))))
}

pub(crate) fn native_file_exists(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.exists()))))
}

pub(crate) fn native_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let path_slot = extract_slot_arg(args, 1);
    let file_obj = heap.get_mut(this_ref)?;
    let Some(path_field) = file_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *path_field = path_slot;
    Ok(None)
}
