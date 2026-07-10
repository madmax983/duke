# Regex `\p{InBlockName}` Unicode-block support — analysis & plan (2026-07-09)

Closing the commons-lang3 3.17.0 regex blocker. This document is a **plan +
reference implementation**, not a landed fix: the regex engine lives entirely
inside `crates/duke-interpreter/src/native.rs`, which was owned by a parallel
session this wave, so the actual wiring must be applied there by that session.
Everything below is designed to be a drop-in.

## Blocker recap

`org/apache/commons/lang3/StringUtils.<clinit>` runs

```java
STRIP_ACCENTS_PATTERN = Pattern.compile("\\p{InCombiningDiacriticalMarks}+");
```

Duke's regex engine rejects the `\p{InCombiningDiacriticalMarks}` Unicode
**block** property, so `Pattern.compile` throws `PatternSyntaxException`, which
propagates out of `<clinit>` before any `StringUtils` method runs. Rendered
blocker (pinned by `commons_lang3_smoke_surfaces_next_missing_capability_explicitly`):

```
JavaException { class_name: "java/util/regex/PatternSyntaxException" }
```

## Where the fix must land (constraint confirmation)

The Java→Rust regex translation is **100% inside `native.rs`**. Confirmed by
grep across `crates/`: no `mod regex`, no `regex.rs`, no separate translation
helper file exists. The only files touching translation/compilation are
`native.rs` (all logic) and `stdlib.rs` (native registration). Relevant symbols,
all in `crates/duke-interpreter/src/native.rs`:

| Symbol | Line (trunk @ this finding) | Role |
| --- | --- | --- |
| `compile_java_regex_with_flags` | ~27963 | Entry point: builds the Rust `regex::Regex` from the Java pattern + flags. **Insertion point.** |
| `translate_java_named_groups` | ~27860 | Existing translation pass: `(?<name>…)` → `(?P<name>…)`. |
| `expand_ascii_case_insensitive` | ~27898 | Existing translation pass for ASCII-only `CASE_INSENSITIVE`. |
| `regex_pattern_syntax_error` | ~27832 | Helper that produces the `PatternSyntaxException`. Reuse for bad block names. |

Registration (`Pattern.compile` → native) is in `stdlib.rs` ~9328. No change
needed there for this fix.

### Exact insertion point

In `compile_java_regex_with_flags`, the non-literal branch currently is:

```rust
let mut source = if flags & PATTERN_LITERAL != 0 {
    regex::escape(pattern)
} else {
    translate_java_named_groups(pattern)   // <-- add block translation right after this
};
```

Insert the property-class translation **immediately after
`translate_java_named_groups`** (and before `expand_ascii_case_insensitive`, so
the ASCII case-folding pass sees the already-expanded `[...]` ranges and leaves
their contents alone):

```rust
} else {
    let named = translate_java_named_groups(pattern);
    translate_property_classes(&named)?   // new pass; returns Result, ? surfaces PatternSyntaxException
};
```

`translate_property_classes` returns `Result<String>`; the `?` propagates a
`regex_pattern_syntax_error(...)` for unknown block names, exactly matching real
Java (`Pattern.compile` throws `PatternSyntaxException` for an unknown block).

## Java→Rust mapping strategy

Rust's `regex` crate (`regex-syntax`) supports Unicode **general categories**
(`\p{Lu}`, `\p{Ll}`, `\p{L}`, `\p{Nd}`, …) and **scripts** (`\p{Latin}`,
`\p{Greek}`, …) natively, with the *same* canonical names Java uses. It does
**not** support Java's `\p{InBlockName}` **block** syntax at all
(`regex-syntax` has no `In` / `blk` / `block=` property). So:

1. **Blocks — `\p{InXxx}` / `\P{InXxx}` (the commons-lang3 blocker).**
   Rust has no block support, so expand each block to its explicit codepoint
   range. `InCombiningDiacriticalMarks` → block "Combining Diacritical Marks" =
   **U+0300–U+036F**:
   - `\p{InCombiningDiacriticalMarks}` → `[\x{0300}-\x{036F}]`
   - `\P{InCombiningDiacriticalMarks}` → `[^\x{0300}-\x{036F}]`
   Block-name matching follows `Character.UnicodeBlock.forName` normalization:
   strip spaces/hyphens/underscores and uppercase, so `InCombiningDiacriticalMarks`,
   `In Combining_Diacritical-Marks`, etc. all resolve to the same block.
   A small static table (block canonical id → `(start, end)`) drives this.
   **Unknown block name ⇒ `regex_pattern_syntax_error("Unknown character block name {Xxx}")`**
   (Java's message shape). This is the priority path — it MUST work.

2. **Scripts — `\p{IsScript}` / `\p{script=Script}`.** Strip the Java `Is`
   prefix and hand the bare name to Rust: `\p{IsLatin}` → `\p{Latin}`,
   `\P{IsGreek}` → `\P{Greek}`. Rust validates the script name; an unknown one
   surfaces as a `PatternSyntaxException` from `builder.build()`. (Natural small
   extension — not required by commons-lang3.)

3. **General categories — `\p{Lu}`, `\p{Ll}`, `\p{L}`, `\p{N}`, …** Pass through
   unchanged; Rust supports them with identical names. (No translation needed;
   they already compile today.)

The scanner must be escaping- and class-aware:
- Track backslash escapes so `\\p` (a literal backslash followed by `p`) is not
  mistaken for the property token `\p`.
- Emit the bracketed form `[\x{..}-\x{..}]` when the block token appears at top
  level, and the bare range `\x{..}-\x{..}` when it appears **inside** an
  existing `[...]` class, so `[\p{InX}a]` becomes `[\x{0300}-\x{036F}a]` rather
  than an unintended nested class. (Rust does accept nested classes, but the bare
  range is the faithful, minimal expansion.) The commons-lang3 pattern
  `\p{InCombiningDiacriticalMarks}+` is top-level, so the bracketed form is the
  critical path.

## Suggested reference implementation

A complete, tested reference implementation of `translate_property_classes`
(plus the block table and name-normalization) lives in
`crates/duke-interpreter/tests/regex_unicode_blocks.rs`. Its unit tests compile
the translated output with the same `regex` crate Duke uses and assert positive
matches, negative matches, and unknown-block → error. The natives session can
lift the `translate_property_classes` / `normalize_block_name` / `UNICODE_BLOCKS`
items straight into `native.rs` and wire the one call shown above.

Pseudocode outline (see the test file for the full, compiling version):

```rust
fn translate_property_classes(pattern: &str) -> Result<String> {
    // scan chars; track `escaped` and `class_depth`.
    // on `\p{...}` or `\P{...}`:
    //   name = contents; negated = (P is uppercase)
    //   if name starts_with "In":
    //       block = normalize_block_name(&name[2..])
    //       (start, end) = UNICODE_BLOCKS.get(block)
    //           .ok_or_else(|| regex_pattern_syntax_error(
    //               format!("Unknown character block name {{{}}}", &name[2..])))?
    //       emit range: top-level -> "[^?\x{start}-\x{end}]", in-class -> bare (no neg support in-class)
    //   else if name starts_with "Is":
    //       emit "\p{" or "\P{" + &name[2..] + "}"   // scripts / binary props -> Rust validates
    //   else:
    //       emit the token unchanged                  // general categories, POSIX, etc.
    // otherwise copy the char verbatim.
}
```

`UNICODE_BLOCKS` minimal seed (extend as libraries demand; the commons-lang3
blocker only needs the first row):

| Java `In…` name | Block | Range |
| --- | --- | --- |
| `InCombiningDiacriticalMarks` | Combining Diacritical Marks | U+0300–U+036F |
| `InBasicLatin` | Basic Latin | U+0000–U+007F |
| `InLatin-1Supplement` | Latin-1 Supplement | U+0080–U+00FF |
| `InGreekAndCoptic` | Greek and Coptic | U+0370–U+03FF |
| `InCyrillic` | Cyrillic | U+0400–U+04FF |

## Verification once wired

- Un-ignore / confirm `commons_lang3_smoke_runs_real_jar_bytecode` advances past
  `StringUtils.<clinit>`.
- The regex-property test file's ignored end-to-end test
  (`regex_unicode_block_pattern_executes_end_to_end`) drives a standalone Java
  fixture and can be un-ignored.

## commons-lang3 next frontier (unchanged prediction)

Clearing the regex gap does **not** finish commons-lang3. The next blocker chain
(from `docs/findings/2026-07-09-oss-smoke-widening.md`, `ArrayUtils.add` path) is
**native.rs / stdlib.rs territory — report only, do not implement here**:

1. `java/lang/reflect/Array.newInstance(Ljava/lang/Class;I)Ljava/lang/Object;`
   (`ArrayUtils.add(int[], int)` → `copyArrayGrow1`).
2. `java/lang/reflect/Array.getLength(Ljava/lang/Object;)I`.
3. `java/lang/reflect/Array.set(Ljava/lang/Object;ILjava/lang/Object;)V`.

`java/lang/reflect/Array` has **no** natives registered today, so
`Array.newInstance` is the expected new frontier once the regex blocker clears.
Until the regex fix lands in `native.rs`, the commons-lang3 smoke still stops at
the `PatternSyntaxException`, so the existing pin
`commons_lang3_smoke_surfaces_next_missing_capability_explicitly` stays valid and
was intentionally left unchanged.
