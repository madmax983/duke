1. **Optimize String concatenation in `native_string_concat`**
   - The function currently uses `format!("{s1}{s2}")` to concatenate two strings, which allocates a new string buffer inside `format!`, formats the arguments, and returns it.
   - We can optimize this by pre-allocating a `String` with the exact required capacity and appending the strings:
     ```rust
     let mut combined = String::with_capacity(s1.len() + s2.len());
     combined.push_str(&s1);
     combined.push_str(&s2);
     let r = heap.allocate_string(combined);
     ```
   - This eliminates the intermediate allocation and parsing overhead of `format!`, which is a common hotspot in interpreters.
   - Add `// ⚡ Bolt: Eliminate intermediate format! allocation` comment.
   - Ensure the tests pass.
