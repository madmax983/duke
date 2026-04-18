cat << 'INNER_EOF' > src_main_patch_all_fixed.py
import re
import sys

content = sys.stdin.read()

# 1. native_hashmap_put_if_absent
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\s*if let Some\(i\) = find_hashmap_entry_index\(&fields, &key, heap\) {\n\s*// Key already present — return existing value\.\n\s*return Ok\(Some\(fields\[i \+ 1\]\)\);\n\s*}",
    """let existing = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap).map(|i| fields[i + 1])
    };
    if let Some(val) = existing {
        return Ok(Some(val));
    }""",
    content
)

# 2. native_hashmap_put
content = re.sub(
    r"// Clone fields to release the immutable borrow before mutating\.\n\s*let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\n\s*if let Some\(i\) = find_hashmap_entry_index\(&fields, &key, heap\) {\n\s*let old = fields\[i \+ 1\];\n\s*heap\.get_mut\(this_ref\)\?\.fields\[i \+ 1\] = val;\n\s*return Ok\(Some\(old\)\);\n\s*}",
    """// Use a scoped borrow to find the index without cloning.
    let existing_idx = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap)
    };

    if let Some(i) = existing_idx {
        let old = heap.get(this_ref)?.fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = val;
        return Ok(Some(old));
    }""",
    content
)

# 3. native_hashmap_get_or_default
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\n\s*Ok\(Some\(\n\s*find_hashmap_entry_index\(&fields, &key, heap\)\.map_or\(default, \|i\| fields\[i \+ 1\]\),\n\s*\)\)",
    """let val = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap).map_or(default, |i| fields[i + 1])
    };
    Ok(Some(val))""",
    content
)

# 4. native_hashmap_get
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\n\s*Ok\(Some\(\n\s*find_hashmap_entry_index\(&fields, &key, heap\)\n\s*\.map_or\(Slot::Reference\(None\), \|i\| fields\[i \+ 1\]\),\n\s*\)\)",
    """let val = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap).map_or(Slot::Reference(None), |i| fields[i + 1])
    };
    Ok(Some(val))""",
    content
)

# 5. native_hashmap_contains_key
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\n\s*if find_hashmap_entry_index\(&fields, &key, heap\)\.is_some\(\) {\n\s*Ok\(Some\(Slot::Int\(1\)\)\)\n\s*} else {\n\s*Ok\(Some\(Slot::Int\(0\)\)\)\n\s*}",
    """let contains = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap).is_some()
    };
    if contains {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }""",
    content
)

# 6. native_hashmap_remove
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\n\s*if let Some\(i\) = find_hashmap_entry_index\(&fields, &key, heap\) {\n\s*let old_val = fields\[i \+ 1\];",
    """let found = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap).map(|i| (i, fields[i + 1]))
    };

    if let Some((i, old_val)) = found {""",
    content
)

# 7. native_hashset_add
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\s*// fields\[0\] = size, fields\[1\.\.\] = elements\n\s*if find_hashset_entry_index\(&fields, &element, heap\)\.is_some\(\) {\n\s*return Ok\(Some\(Slot::Int\(0\)\)\); // duplicate\n\s*}",
    """let contains = {
        let fields = &heap.get(this_ref)?.fields;
        // fields[0] = size, fields[1..] = elements
        find_hashset_entry_index(fields, &element, heap).is_some()
    };
    if contains {
        return Ok(Some(Slot::Int(0))); // duplicate
    }""",
    content
)

# 8. native_hashset_contains
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\n\s*if find_hashset_entry_index\(&fields, &element, heap\)\.is_some\(\) {\n\s*Ok\(Some\(Slot::Int\(1\)\)\)\n\s*} else {\n\s*Ok\(Some\(Slot::Int\(0\)\)\)\n\s*}",
    """let contains = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashset_entry_index(fields, &element, heap).is_some()
    };
    if contains {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }""",
    content
)

# 9. native_hashset_remove
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\n\s*if let Some\(i\) = find_hashset_entry_index\(&fields, &element, heap\) {",
    """let found_idx = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashset_entry_index(fields, &element, heap)
    };

    if let Some(i) = found_idx {""",
    content
)

# 10. native_localdatetime_plus_days
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\s*let epoch = match fields\.first\(\) {\n\s*Some\(Slot::Int\(v\)\) => \*v,\n\s*_ => 0,\n\s*};\n\s*let new_epoch = epoch\.saturating_add\(days as i32\);\n\s*let r = heap\.allocate\(\"java/time/LocalDateTime\"\.to_string\(\), 5\);\n\s*heap\.get_mut\(r\)\?\.fields\[0\] = Slot::Int\(new_epoch\);\n\s*for i in 1\.\.5 {\n\s*heap\.get_mut\(r\)\?\.fields\[i\] = fields\.get\(i\)\.copied\(\)\.unwrap_or\(Slot::Int\(0\)\);\n\s*}",
    """let (epoch, f1, f2, f3, f4) = {
        let fields = &heap.get(this_ref)?.fields;
        let epoch = match fields.first() {
            Some(Slot::Int(v)) => *v,
            _ => 0,
        };
        (epoch, fields.get(1).copied().unwrap_or(Slot::Int(0)),
         fields.get(2).copied().unwrap_or(Slot::Int(0)),
         fields.get(3).copied().unwrap_or(Slot::Int(0)),
         fields.get(4).copied().unwrap_or(Slot::Int(0)))
    };
    let new_epoch = epoch.saturating_add(days as i32);
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    heap.get_mut(r)?.fields[0] = Slot::Int(new_epoch);
    heap.get_mut(r)?.fields[1] = f1;
    heap.get_mut(r)?.fields[2] = f2;
    heap.get_mut(r)?.fields[3] = f3;
    heap.get_mut(r)?.fields[4] = f4;""",
    content
)

# 11. native_localdatetime_with_hour
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\s*let r = heap\.allocate\(\"java/time/LocalDateTime\"\.to_string\(\), 5\);\n\s*for i in 0\.\.5 {\n\s*heap\.get_mut\(r\)\?\.fields\[i\] = fields\.get\(i\)\.copied\(\)\.unwrap_or\(Slot::Int\(0\)\);\n\s*}",
    """let (f0, f1, f2, f3, f4) = {
        let fields = &heap.get(this_ref)?.fields;
        (fields.first().copied().unwrap_or(Slot::Int(0)),
         fields.get(1).copied().unwrap_or(Slot::Int(0)),
         fields.get(2).copied().unwrap_or(Slot::Int(0)),
         fields.get(3).copied().unwrap_or(Slot::Int(0)),
         fields.get(4).copied().unwrap_or(Slot::Int(0)))
    };
    let r = heap.allocate("java/time/LocalDateTime".to_string(), 5);
    heap.get_mut(r)?.fields[0] = f0;
    heap.get_mut(r)?.fields[1] = f1;
    heap.get_mut(r)?.fields[2] = f2;
    heap.get_mut(r)?.fields[3] = f3;
    heap.get_mut(r)?.fields[4] = f4;""",
    content
)

# 12. native_hashmap_remove (the one with 3 args)
content = re.sub(
    r"let fields = heap\.get\(this_ref\)\?\.fields\.clone\(\);\n\s*if let Some\(i\) = find_hashmap_entry_index\(&fields, &key, heap\) {\n\s*let actual_val = fields\[i \+ 1\];\n\s*if slots_equal\(&actual_val, &expected_val, heap\) {\n\s*native_hashmap_remove\(&\[Slot::Reference\(Some\(this_ref\)\), key\], heap, out, control\)\?;\n\s*return Ok\(Some\(Slot::Int\(1\)\)\);\n\s*}\n\s*}",
    """let found = {
        let fields = &heap.get(this_ref)?.fields;
        find_hashmap_entry_index(fields, &key, heap).map(|i| (i, fields[i + 1]))
    };
    if let Some((_i, actual_val)) = found
        && slots_equal(&actual_val, &expected_val, heap)
    {
        native_hashmap_remove(&[Slot::Reference(Some(this_ref)), key], heap, out, control)?;
        return Ok(Some(Slot::Int(1)));
    }""",
    content
)

print(content)
INNER_EOF
cat crates/duke-interpreter/src/native.rs | python3 src_main_patch_all_fixed.py > native_patched_all_fixed.rs
cp native_patched_all_fixed.rs crates/duke-interpreter/src/native.rs
cargo clippy --all-targets --all-features -- -D warnings && cargo test
