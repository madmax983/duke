import java.util.regex.*;
import java.util.Optional;
import java.util.HashMap;

public class Phase39Test {

    // ---- java.util.regex.Pattern / Matcher ----

    static int testPatternMatchesStatic() {
        return Pattern.matches("\\d+", "123") ? 1 : 0;  // 1
    }

    static int testPatternMatchesFails() {
        return Pattern.matches("\\d+", "abc") ? 1 : 0;  // 0
    }

    static int testMatcherFind() {
        Pattern p = Pattern.compile("\\d+");
        Matcher m = p.matcher("abc123def");
        return m.find() ? 1 : 0;  // 1
    }

    static int testMatcherGroup() {
        Pattern p = Pattern.compile("\\d+");
        Matcher m = p.matcher("abc123def");
        m.find();
        return m.group().equals("123") ? 1 : 0;  // 1
    }

    static int testMatcherFindAll() {
        Pattern p = Pattern.compile("[a-z]+");
        Matcher m = p.matcher("abc123def456ghi");
        int count = 0;
        while (m.find()) count++;
        return count;  // 3
    }

    static int testMatcherMatches() {
        Pattern p = Pattern.compile("hello.*");
        Matcher m = p.matcher("hello world");
        return m.matches() ? 1 : 0;  // 1
    }

    static int testMatcherStart() {
        Pattern p = Pattern.compile("\\d+");
        Matcher m = p.matcher("abc123");
        m.find();
        return m.start();  // 3
    }

    static int testMatcherEnd() {
        Pattern p = Pattern.compile("\\d+");
        Matcher m = p.matcher("abc123");
        m.find();
        return m.end();  // 6
    }

    static int testMatcherReplaceAll() {
        Pattern p = Pattern.compile("\\d");
        Matcher m = p.matcher("a1b2c3");
        return m.replaceAll("X").equals("aXbXcX") ? 1 : 0;  // 1
    }

    static int testMatcherReplaceFirst() {
        Pattern p = Pattern.compile("\\d");
        Matcher m = p.matcher("a1b2c3");
        return m.replaceFirst("X").equals("aXb2c3") ? 1 : 0;  // 1
    }

    // ---- String regex methods ----

    static int testStringMatches() {
        return "hello123".matches(".*\\d+") ? 1 : 0;  // 1
    }

    static int testStringMatchesFull() {
        // matches() does full-string match in Java
        return "hello".matches("he.*") ? 1 : 0;  // 1
    }

    static int testStringReplaceAll() {
        return "a1b2c3".replaceAll("\\d", "X").equals("aXbXcX") ? 1 : 0;  // 1
    }

    static int testStringReplaceFirst() {
        return "a1b2c3".replaceFirst("\\d", "X").equals("aXb2c3") ? 1 : 0;  // 1
    }

    static int testStringSplitRegex() {
        String[] parts = "one,two,,three".split(",");
        return parts.length;  // 4 (empty string between commas)
    }

    // ---- Optional extensions ----

    static int testOptionalMap() {
        Optional opt = Optional.of("hello");
        Optional mapped = opt.map(s -> ((String) s).toUpperCase());
        return ((String) mapped.get()).length();  // 5
    }

    static int testOptionalMapEmpty() {
        Optional opt = Optional.empty();
        Optional mapped = opt.map(s -> ((String) s).toUpperCase());
        return mapped.isPresent() ? 1 : 0;  // 0
    }

    static int testOptionalFilter() {
        Optional opt = Optional.of("hello");
        Optional filtered = opt.filter(s -> ((String) s).length() > 3);
        return filtered.isPresent() ? 1 : 0;  // 1
    }

    static int testOptionalFilterDrop() {
        Optional opt = Optional.of("hi");
        Optional filtered = opt.filter(s -> ((String) s).length() > 3);
        return filtered.isPresent() ? 0 : 1;  // 1 (dropped)
    }

    static int testOptionalIfPresent() {
        String[] result = {""};
        Optional.of("hello").ifPresent(v -> result[0] = (String) v);
        return result[0].length();  // 5
    }

    static int testOptionalOrElseGet() {
        Optional opt = Optional.empty();
        String s = (String) opt.orElseGet(() -> "default");
        return s.length();  // 7
    }

    static int testOptionalOrElseGetPresent() {
        Optional opt = Optional.of("hi");
        String s = (String) opt.orElseGet(() -> "default");
        return s.length();  // 2
    }

    // ---- HashMap extensions ----

    static int testHashMapCompute() {
        HashMap map = new HashMap();
        map.put("a", Integer.valueOf(1));
        map.compute("a", (k, v) -> Integer.valueOf(((Integer) v).intValue() + 10));
        return ((Integer) map.get("a")).intValue();  // 11
    }

    static int testHashMapComputeAbsent() {
        HashMap map = new HashMap();
        map.compute("a", (k, v) -> Integer.valueOf(42));
        return ((Integer) map.get("a")).intValue();  // 42
    }

    static int testHashMapMerge() {
        HashMap map = new HashMap();
        map.put("a", Integer.valueOf(5));
        map.merge("a", Integer.valueOf(3),
            (v1, v2) -> Integer.valueOf(((Integer) v1).intValue() + ((Integer) v2).intValue()));
        return ((Integer) map.get("a")).intValue();  // 8
    }

    static int testHashMapMergeAbsent() {
        HashMap map = new HashMap();
        map.merge("a", Integer.valueOf(42),
            (v1, v2) -> Integer.valueOf(((Integer) v1).intValue() + ((Integer) v2).intValue()));
        return ((Integer) map.get("a")).intValue();  // 42
    }
}
