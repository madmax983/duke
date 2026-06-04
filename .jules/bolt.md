**Avoid Byte Indexing for General Text**
**Learning:** Replacing `chars()` with `as_bytes()` iteration for parsing strings (like `split_properties_lines`) will mangle non-ASCII text if `c as char` is used on raw UTF-8 bytes.
**Action:** Only use `as_bytes()` iteration for strictly ASCII structures like JVM descriptors. For text that may contain UTF-8 (like property values), stick to `chars()`.
