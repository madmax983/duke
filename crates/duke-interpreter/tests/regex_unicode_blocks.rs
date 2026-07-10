//! Tests + reference implementation for Java `\p{InBlockName}` Unicode-block
//! regex support — the commons-lang3 3.17.0 `StringUtils.<clinit>` blocker
//! (`Pattern.compile("\\p{InCombiningDiacriticalMarks}+")`).
//!
//! The live regex engine lives entirely inside
//! `crates/duke-interpreter/src/native.rs` (`compile_java_regex_with_flags` and
//! its translation passes), which was owned by a parallel session this wave and
//! must not be edited here. So this file carries a *reference* translation
//! (`translate_property_classes`) that the natives session can lift verbatim
//! into `native.rs`, plus unit tests that validate the Java→Rust mapping against
//! the same `regex` crate Duke compiles patterns with.
//!
//! See `docs/findings/2026-07-09-regex-unicode-blocks.md` for the wiring plan.

use std::path::PathBuf;

use duke_gc::Heap;
use duke_interpreter::{ClassRegistry, bootstrap_stdlib, execute_class_to_completion};
use duke_loader::BootstrapLoader;
use duke_runtime::Slot;

// ---------------------------------------------------------------------------
// Reference implementation (drop-in target for native.rs).
//
// In native.rs the error path should call `regex_pattern_syntax_error(msg)`
// (which raises java/util/regex/PatternSyntaxException); here we surface the
// same message as a plain `String` so the test crate stays self-contained.
// ---------------------------------------------------------------------------

/// Unicode block table: normalized Java `In…` name → inclusive codepoint range.
/// Seeded with the commons-lang3 blocker plus a few neighbours; extend as more
/// libraries demand blocks. `Character.UnicodeBlock.forName` normalizes names by
/// stripping spaces/hyphens/underscores and uppercasing, which `normalize_block_name`
/// mirrors, so every key here is already normalized.
const UNICODE_BLOCKS: &[(&str, u32, u32)] = &[
    ("COMBININGDIACRITICALMARKS", 0x0300, 0x036F),
    ("BASICLATIN", 0x0000, 0x007F),
    ("LATIN1SUPPLEMENT", 0x0080, 0x00FF),
    ("GREEKANDCOPTIC", 0x0370, 0x03FF),
    ("CYRILLIC", 0x0400, 0x04FF),
];

/// Normalize a Java Unicode block name the way `Character.UnicodeBlock.forName`
/// does: drop spaces, hyphens and underscores, then uppercase.
fn normalize_block_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_uppercase)
        .collect()
}

/// Translate one `\p{name}` / `\P{name}` property token into Rust regex syntax.
/// `negated` is true for `\P`; `in_class` is true when the token sits inside an
/// existing `[...]` class. Returns `Err(message)` for an unknown block name.
fn translate_one_property(name: &str, negated: bool, in_class: bool) -> Result<String, String> {
    if let Some(block) = name.strip_prefix("In") {
        // Unicode block: Rust has no block support, so expand to a codepoint range.
        let norm = normalize_block_name(block);
        let (start, end) = UNICODE_BLOCKS
            .iter()
            .find(|(key, _, _)| *key == norm)
            .map(|(_, start, end)| (*start, *end))
            .ok_or_else(|| format!("Unknown character block name {{{block}}}"))?;
        let range = format!("\\x{{{start:04X}}}-\\x{{{end:04X}}}");
        if negated {
            // `[^...]` works both at top level and nested inside another class.
            Ok(format!("[^{range}]"))
        } else if in_class {
            Ok(range)
        } else {
            Ok(format!("[{range}]"))
        }
    } else if let Some(script) = name.strip_prefix("Is") {
        // Script or binary property: strip Java's `Is` and let Rust validate.
        let sigil = if negated { 'P' } else { 'p' };
        Ok(format!("\\{sigil}{{{script}}}"))
    } else {
        // General category / POSIX / anything else: Rust uses the same names.
        let sigil = if negated { 'P' } else { 'p' };
        Ok(format!("\\{sigil}{{{name}}}"))
    }
}

/// Rewrite Java `\p{…}` / `\P{…}` property classes into Rust-regex-compatible
/// syntax, translating Unicode blocks (`\p{InXxx}`) to explicit codepoint ranges
/// and stripping the `Is` script prefix. Escaping- and class-aware. Unknown
/// block names return `Err`, matching Java's `PatternSyntaxException`.
fn translate_property_classes(pattern: &str) -> Result<String, String> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len());
    let mut i = 0;
    let mut class_depth: usize = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            let next = chars.get(i + 1).copied();
            if matches!(next, Some('p' | 'P')) && chars.get(i + 2) == Some(&'{') {
                let sigil = next.expect("checked by matches!");
                let mut j = i + 3;
                let mut name = String::new();
                while j < chars.len() && chars[j] != '}' {
                    name.push(chars[j]);
                    j += 1;
                }
                if j >= chars.len() {
                    // Unterminated `{`; leave verbatim and let the regex builder report it.
                    out.push(c);
                    out.push(sigil);
                    out.push('{');
                    out.push_str(&name);
                    break;
                }
                let translated = translate_one_property(&name, sigil == 'P', class_depth > 0)?;
                out.push_str(&translated);
                i = j + 1;
                continue;
            }
            // Ordinary escape (including `\\`): copy the pair verbatim so an
            // escaped backslash before `p` is not mistaken for a property token.
            out.push(c);
            if let Some(n) = next {
                out.push(n);
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        match c {
            '[' => {
                class_depth += 1;
                out.push(c);
            }
            ']' if class_depth > 0 => {
                class_depth -= 1;
                out.push(c);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Unit tests for the translation + its Java-21 match semantics.
// ---------------------------------------------------------------------------

/// Compile a translated pattern with the same `regex` crate Duke uses.
fn compile(java_pattern: &str) -> regex::Regex {
    let translated = translate_property_classes(java_pattern).expect("translation should succeed");
    regex::Regex::new(&translated)
        .unwrap_or_else(|e| panic!("translated {translated:?} failed to compile: {e}"))
}

#[test]
fn combining_diacritical_marks_block_translates_to_range() {
    assert_eq!(
        translate_property_classes("\\p{InCombiningDiacriticalMarks}+").unwrap(),
        "[\\x{0300}-\\x{036F}]+"
    );
}

#[test]
fn combining_diacritical_marks_block_matches_marks() {
    let re = compile("\\p{InCombiningDiacriticalMarks}+");
    // 'e' + combining acute (U+0301) + combining grave (U+0300) + 'x'.
    let input = "e\u{0301}\u{0300}x";
    let m = re
        .find(input)
        .expect("should match the run of combining marks");
    assert_eq!(m.as_str(), "\u{0301}\u{0300}");
    // Negative: plain ASCII has no combining marks.
    assert!(!re.is_match("abc"));
}

#[test]
fn negated_block_complements_the_range() {
    assert_eq!(
        translate_property_classes("\\P{InCombiningDiacriticalMarks}").unwrap(),
        "[^\\x{0300}-\\x{036F}]"
    );
    let re = compile("\\P{InCombiningDiacriticalMarks}");
    assert!(re.is_match("a"));
    assert!(!re.is_match("\u{0301}"));
}

#[test]
fn block_inside_character_class_emits_bare_range() {
    assert_eq!(
        translate_property_classes("[a\\p{InCombiningDiacriticalMarks}]").unwrap(),
        "[a\\x{0300}-\\x{036F}]"
    );
    let re = compile("[a\\p{InCombiningDiacriticalMarks}]+");
    let m = re.find("a\u{0301}z").expect("should match 'a' + mark");
    assert_eq!(m.as_str(), "a\u{0301}");
}

#[test]
fn unknown_block_name_is_rejected() {
    let err = translate_property_classes("\\p{InNotARealBlockName}").unwrap_err();
    assert!(
        err.contains("Unknown character block name"),
        "expected a Java-style block error, got: {err}"
    );
    assert!(
        err.contains("NotARealBlockName"),
        "error should name the bad block: {err}"
    );
}

#[test]
fn block_name_normalization_is_lenient() {
    // Spaces/hyphens/underscores/case are all ignored, per Character.UnicodeBlock.
    for name in [
        "\\p{InCombiningDiacriticalMarks}",
        "\\p{In Combining Diacritical Marks}",
        "\\p{Incombining_diacritical-marks}",
    ] {
        assert_eq!(
            translate_property_classes(name).unwrap(),
            "[\\x{0300}-\\x{036F}]",
            "name {name:?} should normalize to the same block"
        );
    }
}

#[test]
fn script_is_prefix_is_stripped() {
    assert_eq!(
        translate_property_classes("\\p{IsLatin}+").unwrap(),
        "\\p{Latin}+"
    );
    assert_eq!(
        translate_property_classes("\\P{IsGreek}").unwrap(),
        "\\P{Greek}"
    );
    let re = compile("\\p{IsLatin}+");
    assert!(re.is_match("abc"));
    assert!(!re.is_match("\u{0391}")); // Greek capital alpha
}

#[test]
fn general_categories_pass_through_unchanged() {
    assert_eq!(translate_property_classes("\\p{Lu}").unwrap(), "\\p{Lu}");
    assert_eq!(translate_property_classes("\\p{Ll}+").unwrap(), "\\p{Ll}+");
    let upper = compile("\\p{Lu}");
    assert!(upper.is_match("A"));
    assert!(!upper.is_match("a"));
}

#[test]
fn escaped_backslash_is_not_treated_as_a_property() {
    // `\\p{...}` = a literal backslash followed by literal `p{...}`, not a
    // property token, so it must pass through untouched.
    assert_eq!(
        translate_property_classes("\\\\p{InCombiningDiacriticalMarks}").unwrap(),
        "\\\\p{InCombiningDiacriticalMarks}"
    );
}

#[test]
fn non_property_pattern_is_unchanged() {
    let pat = "(?P<word>[A-Za-z]+)\\d{2,3}";
    assert_eq!(translate_property_classes(pat).unwrap(), pat);
}

// ---------------------------------------------------------------------------
// End-to-end canary through the real Pattern.compile native path.
//
// The native.rs `translate_property_classes` wiring (plus the Matcher UTF-16
// match-index conversion) described in
// docs/findings/2026-07-09-regex-unicode-blocks.md has landed, so this runs by
// default now.
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical workspace root")
}

fn jdk_modules_path() -> Option<PathBuf> {
    let from_java_home = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .map(|path| path.join("lib/modules"));
    let fallbacks = [
        PathBuf::from("/usr/lib/jvm/java-21-openjdk-amd64/lib/modules"),
        PathBuf::from("/usr/lib/jvm/default-java/lib/modules"),
    ];
    from_java_home
        .into_iter()
        .chain(fallbacks)
        .find(|path| path.exists())
}

#[test]
fn regex_unicode_block_pattern_executes_end_to_end() {
    let Some(modules_path) = jdk_modules_path() else {
        eprintln!("skipping: no JDK lib/modules found");
        return;
    };
    let fixtures = repo_root().join("tests/fixtures");
    let loader = BootstrapLoader::new(&modules_path, vec![fixtures]).expect("bootstrap loader");

    let mut registry = ClassRegistry::new();
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    assert!(
        registry
            .ensure_loaded("RegexUnicodeBlockTest", &loader)
            .expect("load RegexUnicodeBlockTest"),
        "fixture should load"
    );

    let args_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let main_args = [Slot::Reference(Some(args_ref))];
    let mut output = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut output,
        "RegexUnicodeBlockTest",
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    );
    assert!(result.is_ok(), "main should run end-to-end, got {result:?}");

    let text = String::from_utf8(output).expect("utf8 output");
    // Golden values verified against real Java 21.
    assert!(text.contains("block-match found=true len=2"), "got: {text}");
    assert!(text.contains("no-mark find=false"), "got: {text}");
    assert!(text.contains("bad-block threw=true"), "got: {text}");
}
