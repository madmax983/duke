public class StringCharsTest {
    static int testToCharArrayLength() {
        String s = "hello";
        char[] chars = s.toCharArray();
        return chars.length;  // 5
    }

    static int testManualCharIteration() {
        String s = "abc";
        int sum = 0;
        for (int i = 0; i < s.length(); i++) {
            sum += s.charAt(i);  // 'a'=97, 'b'=98, 'c'=99
        }
        return sum;  // 294
    }

    static int testCodePointAt() {
        String s = "A";
        return s.codePointAt(0);  // 65
    }

    static int testCompareTo() {
        String a = "apple";
        String b = "banana";
        return a.compareTo(b) < 0 ? 1 : 0;  // 1 (a < b)
    }

    static int testValueOfChar() {
        char c = 'Z';
        String s = String.valueOf(c);
        return s.length();  // 1
    }
}
