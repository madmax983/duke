public class StringExtTest {
    static int testStrip() {
        String s = "  hello  ".strip();
        return s.length();  // 5
    }

    static int testStripLeading() {
        String s = "  hi".stripLeading();
        return s.length();  // 2
    }

    static int testStripTrailing() {
        String s = "hi  ".stripTrailing();
        return s.length();  // 2
    }

    static int testIsBlankTrue() {
        return "   ".isBlank() ? 1 : 0;  // 1
    }

    static int testIsBlankFalse() {
        return "hi".isBlank() ? 1 : 0;  // 0
    }

    static int testRepeat() {
        return "ab".repeat(3).length();  // 6
    }

    static int testRepeatZero() {
        return "abc".repeat(0).length();  // 0
    }

    static int testJoin() {
        String result = String.join("-", new String[]{"a", "b", "c"});
        return result.length();  // 5
    }

    static int testIndexOfChar() {
        return "hello".indexOf('l');  // 2
    }

    static int testIndexOfCharMissing() {
        return "hello".indexOf('z');  // -1
    }

    static int testLastIndexOf() {
        return "abcabc".lastIndexOf("bc");  // 4
    }
}
