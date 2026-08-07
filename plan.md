1. **Refactor `format_cp_entry` in `duke/src/main.rs`**
   - **Smell**: `format_cp_entry` takes `&ClassFile` and `&CpEntry` and returns a `String` by matching on all 17 enum variants, allocating strings and calling `format!` for every single entry, taking up 100 lines.
   - **Solution**: Wrap `&ClassFile` and `&CpEntry` in a struct `FormattedCpEntry<'a>`. Implement `std::fmt::Display` for this struct. Use `write!` or `writeln!` instead of `format!`, completely removing the intermediate string allocations and turning a massive God-match into clean formatting logic. Change the single call site in `dump_class_file` to print using `{}` directly.

2. **Refactor `cp_str` and `resolve_class_name` in `duke/src/main.rs`**
   - **Smell**: Deeply nested `and_then` chains to safely extract `&str` from the constant pool.
   - **Solution**: Use `let ... else` guard clauses to early return `None` and flatten the nesting.

3. **Refactor `build_method_entries` in `crates/duke-interpreter/src/native/common.rs`**
   - **Smell**: Deeply nested logic and `match` blocks when reading from `cf.constant_pool` for attributes, names, descriptors, and exception table catch types.
   - **Solution**: Use `let ... else` guard clauses inside the iterators/filter_maps to extract the string values without heavy nesting. For example, replacing:
     ```rust
     let name = match cf.constant_pool.get(m.name_index.0 as usize) {
         Some(Some(CpEntry::Utf8(s))) => s.clone(),
         _ => return None,
     };
     ```
     with
     ```rust
     let Some(Some(CpEntry::Utf8(name))) = cf.constant_pool.get(m.name_index.0 as usize) else { return None; };
     let name = name.clone();
     ```
     and similar extractions to clean up the structure.

4. **Run Checks & Tests**
   - Run `cargo fmt --all`
   - Run `cargo clippy --all-targets --all-features -- -D warnings`
   - Run `cargo test`

5. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**

6. **Request Code Review**
