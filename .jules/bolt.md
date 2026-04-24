**Remove intermediate allocations in String operations**
**Learning:** Using `.chars().take(n).collect()` to truncate strings creates unnecessary new `String` allocations because it consumes the characters and collects them into a fresh buffer. Furthermore, using `.map(f).unwrap_or(a)` triggers a clippy lint which prefers `.map_or(a, f)`.
**Action:** When truncating a string safely at a character boundary, calculate the byte index using `.char_indices().nth(n).map_or(buf.len(), |(i, _)| i)` and truncate the string in place with `.truncate(byte_idx)`.
