import re

with open("crates/duke-interpreter/src/native/java_util_regex.rs", "r") as f:
    content = f.read()

# matcher_find
search1 = """    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let input_slot = fields
        .get(MATCHER_INPUT_FIELD)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let Slot::Reference(Some(input_ref)) = input_slot else {
        return Ok(Some(Slot::Int(0)));
    };
    let pos = match fields.get(MATCHER_POS_FIELD).copied() {
        Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
        _ => 0,
    };"""

replace1 = """    let (pat_ref, input_ref, pos) = {
        let fields = &heap.get(m_ref)?.fields;
        let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
            return Ok(Some(Slot::Int(0)));
        };
        let input_slot = fields
            .get(MATCHER_INPUT_FIELD)
            .copied()
            .unwrap_or(Slot::Reference(None));
        let Slot::Reference(Some(input_ref)) = input_slot else {
            return Ok(Some(Slot::Int(0)));
        };
        let pos = match fields.get(MATCHER_POS_FIELD).copied() {
            Some(Slot::Int(n)) => usize::try_from(n.max(0)).unwrap_or(0),
            _ => 0,
        };
        (pat_ref, input_ref, pos)
    };"""

content = content.replace(search1, replace1)


# matcher_matches
search2 = """    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
        return Ok(Some(Slot::Int(0)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
        return Ok(Some(Slot::Int(0)));
    };"""

replace2 = """    let (pat_ref, input_ref) = {
        let fields = &heap.get(m_ref)?.fields;
        let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
            return Ok(Some(Slot::Int(0)));
        };
        let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
            return Ok(Some(Slot::Int(0)));
        };
        (pat_ref, input_ref)
    };"""

content = content.replace(search2, replace2)

# matcher_replace_all
search3 = """    let fields = heap.get(m_ref)?.fields.clone();
    let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };
    let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
        return Ok(Some(Slot::Reference(None)));
    };"""

replace3 = """    let (pat_ref, input_ref) = {
        let fields = &heap.get(m_ref)?.fields;
        let Some(Slot::Reference(Some(pat_ref))) = fields.get(MATCHER_PATTERN_FIELD).copied() else {
            return Ok(Some(Slot::Reference(None)));
        };
        let Some(Slot::Reference(Some(input_ref))) = fields.get(MATCHER_INPUT_FIELD).copied() else {
            return Ok(Some(Slot::Reference(None)));
        };
        (pat_ref, input_ref)
    };"""

content = content.replace(search3, replace3)


with open("crates/duke-interpreter/src/native/java_util_regex.rs", "w") as f:
    f.write(content)
