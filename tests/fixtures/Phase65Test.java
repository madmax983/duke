import java.util.HashMap;

public class Phase65Test {

    // String.strip() — removes leading and trailing whitespace
    public static int testStringStrip() {
        String s = "  hello world  ";
        return s.strip().length(); // expect 11
    }

    // String.stripLeading()
    public static int testStringStripLeading() {
        String s = "   abc";
        return s.stripLeading().length(); // expect 3
    }

    // String.stripTrailing()
    public static int testStringStripTrailing() {
        String s = "xyz   ";
        return s.stripTrailing().length(); // expect 3
    }

    // String.repeat(int)
    public static int testStringRepeat() {
        String s = "ab";
        return s.repeat(4).length(); // "abababab" = 8
    }

    // String.repeat(0) — empty result
    public static int testStringRepeatZero() {
        String s = "hello";
        return s.repeat(0).length(); // expect 0
    }

    // String.isBlank() — blank string
    public static int testStringIsBlankTrue() {
        String s = "   ";
        return s.isBlank() ? 1 : 0; // expect 1
    }

    // String.isBlank() — non-blank
    public static int testStringIsBlankFalse() {
        String s = "  x  ";
        return s.isBlank() ? 1 : 0; // expect 0
    }

    // Integer.toBinaryString
    public static int testIntegerToBinaryString() {
        String s = Integer.toBinaryString(10); // "1010"
        return s.length(); // expect 4
    }

    // Integer.toHexString
    public static int testIntegerToHexString() {
        String s = Integer.toHexString(255); // "ff"
        return s.length(); // expect 2
    }

    // Integer.toOctalString
    public static int testIntegerToOctalString() {
        String s = Integer.toOctalString(8); // "10"
        return s.length(); // expect 2
    }

    // Integer.toHexString — value check via charAt
    public static int testIntegerToHexStringValue() {
        String s = Integer.toHexString(0xABCD); // "abcd"
        int r = 0;
        if (s.charAt(0) == 'a') r += 1;
        if (s.charAt(1) == 'b') r += 2;
        if (s.charAt(2) == 'c') r += 4;
        if (s.charAt(3) == 'd') r += 8;
        return r; // expect 15
    }

    // HashMap.computeIfAbsent — key absent, computed and stored
    public static int testHashMapComputeIfAbsentMiss() {
        HashMap<String, Integer> map = new HashMap<>();
        map.computeIfAbsent("k", key -> key.length()); // "k".length() = 1
        Object val = map.get("k");
        if (val instanceof Integer) {
            return (Integer) val; // expect 1
        }
        return -1;
    }

    // HashMap.computeIfAbsent — key present, not replaced
    public static int testHashMapComputeIfAbsentHit() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", 42);
        map.computeIfAbsent("x", key -> 99);
        Object val = map.get("x");
        if (val instanceof Integer) {
            return (Integer) val; // expect 42 (unchanged)
        }
        return -1;
    }

    // HashMap.forEach — sums all values
    public static int testHashMapForEach() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", 10);
        map.put("b", 20);
        map.put("c", 30);
        int[] sum = {0};
        map.forEach((k, v) -> sum[0] += v);
        return sum[0]; // expect 60
    }
}
