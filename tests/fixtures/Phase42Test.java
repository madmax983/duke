import java.util.*;
import java.util.stream.*;

public class Phase42Test {

    // ---- Collectors.counting() ----

    static int testCollectorsCounting() {
        List<String> list = Arrays.asList("a", "b", "c");
        long count = list.stream().collect(Collectors.counting());
        return (int) count;  // 3
    }

    // ---- Collectors.groupingBy ----

    @SuppressWarnings("unchecked")
    static int testGroupingBySize() {
        List<String> list = Arrays.asList("a", "bb", "cc", "ddd");
        Map grouped = (Map) ((Stream) list.stream())
            .collect(Collectors.groupingBy((Object s) -> ((String) s).length()));
        return grouped.size();  // 3 distinct lengths
    }

    @SuppressWarnings("unchecked")
    static int testGroupingByCount() {
        List<String> words = Arrays.asList("hi", "ho", "hey");
        Map byLen = (Map) ((Stream) words.stream())
            .collect(Collectors.groupingBy((Object s) -> ((String) s).length()));
        // length 2: ["hi","ho"], length 3: ["hey"]
        List len2 = (List) byLen.get(Integer.valueOf(2));
        return len2.size();  // 2
    }

    // ---- Stream.peek ----

    static int testStreamPeek() {
        int[] count = {0};
        long result = Stream.of("a", "b", "c")
            .peek(x -> count[0]++)
            .count();
        return (int) result + count[0];  // 3 + 3 = 6
    }

    // ---- Stream.toArray ----

    static int testStreamToArray() {
        Object[] arr = Stream.of("a", "b", "c").toArray();
        return arr.length;  // 3
    }

    // ---- Arrays.stream(int[]) ----

    static int testArraysStreamInt() {
        int[] arr = {1, 2, 3, 4, 5};
        return Arrays.stream(arr).sum();  // 15
    }

    static int testArraysStreamIntFilter() {
        int[] arr = {1, 2, 3, 4, 5};
        return (int) Arrays.stream(arr).filter(n -> n % 2 == 0).count();  // 2
    }

    // ---- Math.random() range check ----

    static int testMathRandom() {
        double r = Math.random();
        return (r >= 0.0 && r < 1.0) ? 1 : 0;  // 1
    }

    // ---- Comparator.comparing ----

    static int testComparatorComparing() {
        List<String> list = new ArrayList<>(Arrays.asList("banana", "apple", "cherry"));
        list.sort(Comparator.comparing(s -> (String) s));
        return list.get(0).equals("apple") ? 1 : 0;  // 1
    }

    // ---- HashMap.forEach ----

    static int testHashMapForEach() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(1));
        map.put("b", Integer.valueOf(2));
        map.put("c", Integer.valueOf(3));
        int[] sum = {0};
        map.forEach((k, v) -> sum[0] += ((Integer) v).intValue());
        return sum[0];  // 6
    }

    // ---- String.join with List ----

    static int testStringJoinList() {
        List<String> parts = Arrays.asList("a", "b", "c");
        return String.join(", ", parts).equals("a, b, c") ? 1 : 0;  // 1
    }

    // ---- Collections.unmodifiableList ----

    static int testUnmodifiableList() {
        List<String> base = new ArrayList<>(Arrays.asList("x", "y"));
        List<String> unmod = Collections.unmodifiableList(base);
        return unmod.size();  // 2
    }

    // ---- Integer.compare ----

    static int testIntegerCompare() {
        return Integer.compare(5, 3);  // positive (>0)
    }

    static int testIntegerCompareEqual() {
        return Integer.compare(3, 3);  // 0
    }

    // ---- Long.compare ----

    static int testLongCompare() {
        return Long.compare(5L, 10L) < 0 ? 1 : 0;  // 1
    }
}
