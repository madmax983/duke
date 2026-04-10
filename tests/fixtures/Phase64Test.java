import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;

public class Phase64Test {

    // String.indent(int) — positive n adds leading spaces, result length check
    public static int testStringIndentPositive() {
        String s = "hello\nworld";
        String indented = s.indent(4);
        // "    hello\n    world\n" = 4+5+1 + 4+5+1 = 20 chars
        return indented.length(); // expect 20
    }

    // String.indent(int) — negative n removes leading spaces
    public static int testStringIndentNegative() {
        String s = "    hello\n    world";
        String dedented = s.indent(-2);
        // "  hello\n  world\n" = 2+5+1 + 2+5+1 = 16 chars
        return dedented.length(); // expect 16
    }

    // String.indent(int) — zero n adds trailing newline only
    public static int testStringIndentZero() {
        String s = "abc";
        String result = s.indent(0);
        // "abc\n" — length 4
        return result.length(); // expect 4
    }

    // String.indent verifies prefix character via charAt
    public static int testStringIndentStartsWith() {
        String s = "x";
        String indented = s.indent(3);
        // "   x\n" — first char is ' '
        int r = 0;
        if (indented.charAt(0) == ' ') r += 1;
        if (indented.charAt(1) == ' ') r += 2;
        if (indented.charAt(2) == ' ') r += 4;
        if (indented.charAt(3) == 'x') r += 8;
        return r; // expect 15
    }

    // StringBuilder.setCharAt(int, char)
    public static int testStringBuilderSetCharAt() {
        StringBuilder sb = new StringBuilder("hello");
        sb.setCharAt(0, 'H');
        sb.setCharAt(4, '!');
        String result = sb.toString(); // "Hell!"
        return result.length(); // expect 5
    }

    // StringBuilder.setCharAt — verify character was changed
    public static int testStringBuilderSetCharAtValue() {
        StringBuilder sb = new StringBuilder("abc");
        sb.setCharAt(1, 'X');
        String result = sb.toString(); // "aXc"
        int r = 0;
        if (result.charAt(0) == 'a') r += 1;
        if (result.charAt(1) == 'X') r += 2;
        if (result.charAt(2) == 'c') r += 4;
        return r; // expect 7
    }

    // Collections.disjoint — disjoint lists
    public static int testCollectionsDisjointTrue() {
        ArrayList<Integer> a = new ArrayList<>();
        a.add(1);
        a.add(2);
        a.add(3);
        ArrayList<Integer> b = new ArrayList<>();
        b.add(4);
        b.add(5);
        return Collections.disjoint(a, b) ? 1 : 0; // expect 1 (disjoint)
    }

    // Collections.disjoint — non-disjoint lists
    public static int testCollectionsDisjointFalse() {
        ArrayList<Integer> a = new ArrayList<>();
        a.add(1);
        a.add(2);
        a.add(3);
        ArrayList<Integer> b = new ArrayList<>();
        b.add(3);
        b.add(4);
        return Collections.disjoint(a, b) ? 1 : 0; // expect 0 (not disjoint)
    }

    // HashMap.computeIfPresent — key exists, value replaced
    public static int testHashMapComputeIfPresentHit() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", 10);
        map.computeIfPresent("x", (k, v) -> v + 5);
        Object val = map.get("x");
        if (val instanceof Integer) {
            return (Integer) val; // expect 15
        }
        return -1;
    }

    // HashMap.computeIfPresent — key absent, map unchanged
    public static int testHashMapComputeIfPresentMiss() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("y", 99);
        map.computeIfPresent("z", (k, v) -> v * 2);
        return map.size(); // expect 1 (only "y")
    }
}
