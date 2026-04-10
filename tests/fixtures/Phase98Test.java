import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase98Test {

    // ---- DoubleStream operations ----
    public static int testDoubleStream() {
        double sum = DoubleStream.of(1.5, 2.5, 3.0, 4.0)
            .filter(d -> d >= 2.5)
            .sum();
        return (int) sum; // 2.5+3.0+4.0 = 9.5 → 9
    }

    // ---- Map.replaceAll ----
    public static int testMapReplaceAll() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1); m.put("b", 2); m.put("c", 3);
        m.replaceAll((k, v) -> v * 10);
        return m.get("a") + m.get("b") + m.get("c"); // 10+20+30 = 60
    }

    // ---- Collectors.groupingBy + counting ----
    public static int testGroupingByCount() {
        List<String> words = Arrays.asList("cat", "dog", "car", "deer", "duck", "cow");
        Map<Character, Long> byFirstChar = words.stream()
            .collect(Collectors.groupingBy(s -> s.charAt(0), Collectors.counting()));
        return byFirstChar.get('c').intValue() + byFirstChar.get('d').intValue();
        // c: cat, car, cow = 3; d: dog, deer, duck = 3 → 6
    }

    // ---- Stream.of with flatMap ----
    public static int testStreamFlatMapInt() {
        return Stream.of("hello", "world", "java")
            .flatMapToInt(String::chars)
            .filter(c -> c == 'l')
            .sum(); // 'l'=108, appears in "hello"(2) + "world"(1) = 3 times → 3*108=324
    }

    // ---- Comparator.reversed ----
    public static int testComparatorReversed() {
        List<Integer> nums = new ArrayList<>(Arrays.asList(3, 1, 4, 1, 5, 9));
        nums.sort(Comparator.naturalOrder());
        int first = nums.get(0); // 1
        nums.sort(Comparator.reverseOrder());
        int last = nums.get(0); // 9
        return first + last; // 10
    }

    // ---- Multiple interfaces ----
    interface Printable { String print(); }
    interface Saveable { String save(); }
    static class Document implements Printable, Saveable {
        String content;
        Document(String content) { this.content = content; }
        public String print() { return "PRINT:" + content; }
        public String save() { return "SAVE:" + content; }
    }

    public static int testMultipleInterfaces() {
        Document d = new Document("hello");
        Printable p = d;
        Saveable s = d;
        return p.print().length() + s.save().length(); // 11 + 10 = 21
    }

    // ---- String.indexOf variants ----
    public static int testStringIndexOf() {
        String s = "hello world hello";
        int first = s.indexOf("hello");           // 0
        int second = s.indexOf("hello", 1);       // 12
        int last = s.lastIndexOf("hello");        // 12
        return first + second + last; // 0 + 12 + 12 = 24
    }

    // ---- Exception hierarchy catch ----
    public static int testExceptionHierarchy() {
        int sum = 0;
        for (int i = 0; i < 3; i++) {
            try {
                if (i == 0) throw new IllegalArgumentException("iae");
                if (i == 1) throw new NullPointerException("npe");
                if (i == 2) throw new ArrayIndexOutOfBoundsException("aioobe");
            } catch (IllegalArgumentException e) {
                sum += 1;
            } catch (RuntimeException e) {
                sum += 10;
            }
        }
        return sum; // 1 + 10 + 10 = 21
    }

    // ---- Nested lambdas with capture ----
    public static int testNestedLambdaCapture() {
        int base = 100;
        Function<Integer, Function<Integer, Integer>> adder = x -> y -> base + x + y;
        return adder.apply(5).apply(3); // 100 + 5 + 3 = 108
    }

    // ---- Map.values().stream() ----
    public static int testMapValuesStream() {
        Map<String, Integer> scores = new HashMap<>();
        scores.put("A", 85); scores.put("B", 92); scores.put("C", 78); scores.put("D", 95);
        return (int) scores.values().stream()
            .filter(s -> s >= 90)
            .count(); // 92 and 95 = 2
    }
}
