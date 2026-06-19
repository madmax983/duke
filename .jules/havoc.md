## 2024-05-24 - [Classfile Annotation parsing OOM due to deep recursion]
**The Trigger:** A class file containing a deeply nested `RuntimeVisibleAnnotations` attribute (e.g. nested arrays inside annotations inside annotations).
**The Stack Trace:** The process panics with `stack overflow` or crashes due to OOM when allocating vectors at each recursion level, or exceeds system limits.
**Reproduction:** Run `cargo test --test havoc_classfile_annotation_oom` (which we added and fixed).
**Comment:** "You assumed the JVM specification limit of annotation depth was naturally bounded by the class file size, but deep nesting of `[` arrays with 1 element allows exponential stack growth compared to byte size. You were wrong."
**Regex Panic Avoidance**
**Learning:** `regex::Regex::new` panics when `regex::escape` fails, which only happens if the Regex compiler rejects the string but we bypass `map_err`. We must handle the error properly by safely unwrapping or returning the error when `regex::Regex::new` encounters an invalid or too large regex.
**Action:** When emulating Java's string split, gracefully propagate the `PatternSyntaxException` rather than calling `.unwrap()`.
