import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase89Test {

    // ---- Map.putIfAbsent ----
    public static int testMapPutIfAbsent() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        m.putIfAbsent("a", 99);   // should NOT overwrite
        m.putIfAbsent("b", 2);    // should insert
        return m.get("a") + m.get("b"); // 1 + 2 = 3
    }

    // ---- Map.merge ----
    public static int testMapMerge() {
        Map<String, Integer> m = new HashMap<>();
        m.put("x", 10);
        m.merge("x", 5, Integer::sum);  // 10 + 5 = 15
        m.merge("y", 7, Integer::sum);  // new key: 7
        return m.get("x") + m.get("y"); // 15 + 7 = 22
    }

    // ---- Comparator.comparing ----
    public static int testComparatorComparing() {
        List<String> words = new ArrayList<>(Arrays.asList("banana", "apple", "cherry", "date"));
        words.sort(Comparator.comparing(String::length));
        // lengths: banana=6, apple=5, cherry=6, date=4 → sorted by length: date(4), apple(5), banana(6), cherry(6)
        return words.get(0).length() + words.get(1).length(); // 4 + 5 = 9
    }

    // ---- Collections.frequency ----
    public static int testCollectionsFrequency() {
        List<String> list = Arrays.asList("a", "b", "a", "c", "a");
        return Collections.frequency(list, "a"); // 3
    }

    // ---- String.chars() stream ----
    public static int testStringCharsStream() {
        String s = "Hello, World!";
        long count = s.chars()
            .filter(c -> c == 'l')
            .count();
        return (int) count; // 3
    }

    // ---- List.contains ----
    public static int testListContains() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        int result = 0;
        if (list.contains(3)) result += 10;
        if (!list.contains(6)) result += 5;
        return result; // 15
    }

    // ---- Nested generic method call ----
    public static int testOptionalMap() {
        Optional<String> opt = Optional.of("hello");
        Optional<Integer> len = opt.map(String::length);
        return len.orElse(0); // 5
    }

    // ---- String.valueOf variants ----
    public static int testStringValueOf() {
        String i = String.valueOf(42);
        String d = String.valueOf(3.14);
        String b = String.valueOf(true);
        return i.length() + d.length() + b.length(); // 2 + 4 + 4 = 10
    }

    // ---- IntStream.range().sum() ----
    public static int testIntStreamRange() {
        int sum = IntStream.range(1, 6).sum(); // 1+2+3+4+5=15
        return sum;
    }

    // ---- Map iteration with forEach ----
    public static int testMapForEach() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        m.put("b", 2);
        m.put("c", 3);
        int[] total = {0};
        m.forEach((k, v) -> total[0] += v);
        return total[0]; // 6
    }
}
