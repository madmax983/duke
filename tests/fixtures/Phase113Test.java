import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase113Test {

    // ---- LinkedHashMap insertion order ----
    public static int testLinkedHashMap() {
        Map<String, Integer> m = new LinkedHashMap<>();
        m.put("c", 3); m.put("a", 1); m.put("b", 2);
        List<Integer> vals = new ArrayList<>(m.values());
        return vals.get(0) + vals.get(1) + vals.get(2); // 3+1+2 = 6 (insertion order)
    }

    // ---- Collections.swap ----
    public static int testCollectionsSwap() {
        List<Integer> list = new ArrayList<>(Arrays.asList(10, 20, 30, 40, 50));
        Collections.swap(list, 0, 4);
        return list.get(0) + list.get(4); // 50 + 10 = 60
    }

    // ---- Stream.generate().limit() ----
    public static int testStreamGenerate() {
        int[] counter = {0};
        return (int) Stream.generate(() -> { counter[0]++; return counter[0]; })
            .limit(5)
            .filter(n -> n % 2 != 0)
            .count(); // 1,2,3,4,5 → odd: 1,3,5 = 3
    }

    // ---- Collectors.averagingInt ----
    public static int testCollectorsAveragingInt() {
        double avg = Stream.of(1, 2, 3, 4, 5)
            .collect(Collectors.averagingInt(Integer::intValue));
        return (int) avg; // 3
    }

    // ---- Map.computeIfAbsent ----
    public static int testMapComputeIfAbsent() {
        Map<String, List<Integer>> m = new HashMap<>();
        m.computeIfAbsent("a", k -> new ArrayList<>()).add(1);
        m.computeIfAbsent("a", k -> new ArrayList<>()).add(2);
        m.computeIfAbsent("b", k -> new ArrayList<>()).add(3);
        return m.get("a").size() + m.get("b").size(); // 2 + 1 = 3
    }

    // ---- Integer.bitCount ----
    public static int testIntegerBitCount() {
        return Integer.bitCount(255) + Integer.bitCount(0) + Integer.bitCount(7);
        // 8 + 0 + 3 = 11
    }

    // ---- String.formatted (Java 15+) — just use format ----
    public static int testStringFormatMultiple() {
        String s = String.format("%-5s|%05d|%.2f", "hi", 42, 3.14);
        return s.length(); // "hi   |00042|3.14" = 5+1+5+1+4 = 16
    }

    // ---- Stream.of empty ----
    public static int testStreamEmpty() {
        return (int) Stream.empty().count(); // 0
    }

    // ---- List.set ----
    public static int testListSet() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        list.set(2, 99);
        return list.stream().mapToInt(Integer::intValue).sum(); // 1+2+99+4+5 = 111
    }

    // ---- Collectors.groupingBy with downstream ----
    public static int testGroupingByDownstream() {
        List<String> words = Arrays.asList("apple", "ant", "ball", "bat", "bear", "cat");
        Map<Character, Long> grouped = words.stream()
            .collect(Collectors.groupingBy(s -> s.charAt(0), Collectors.counting()));
        return grouped.get('a').intValue() + grouped.get('b').intValue() + grouped.get('c').intValue();
        // a:2, b:3, c:1 = 6
    }
}
