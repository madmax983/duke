import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.regex.PatternSyntaxException;

// Fixture for the \p{InBlockName} Unicode-block regex support (commons-lang3
// STRIP_ACCENTS_PATTERN blocker). Exercised by the ignored end-to-end test in
// crates/duke-interpreter/tests/regex_unicode_blocks.rs. Requires the pending
// native.rs translate_property_classes wiring described in
// docs/findings/2026-07-09-regex-unicode-blocks.md.
public class RegexUnicodeBlockTest {
    public static void main(String[] args) {
        // Positive: the Combining Diacritical Marks block (U+0300-U+036F) matches
        // the combining acute (U+0301) and combining grave (U+0300) after 'e'.
        Pattern p = Pattern.compile("\\p{InCombiningDiacriticalMarks}+");
        String input = "e\u0301\u0300x";
        Matcher m = p.matcher(input);
        boolean found = m.find();
        int len = found ? (m.end() - m.start()) : 0;
        System.out.println("block-match found=" + found + " len=" + len);

        // Negative: plain ASCII contains no combining marks.
        Matcher m2 = p.matcher("abc");
        System.out.println("no-mark find=" + m2.find());

        // Bad block name must throw PatternSyntaxException (matching real Java).
        boolean threw = false;
        try {
            Pattern.compile("\\p{InNotARealBlockName}");
        } catch (PatternSyntaxException e) {
            threw = true;
        }
        System.out.println("bad-block threw=" + threw);
    }
}
