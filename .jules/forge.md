**Extract God Function**
**Learning:** `apply_format_width` suffered from Boolean Blindness, taking multiple un-typed boolean flags for alignment behavior, reducing clarity at call sites.
**Action:** Replaced ambiguous boolean flags with a strictly typed enum (`FormatAlignment`), using Guard Clauses internally, to improve clarity and type safety.
