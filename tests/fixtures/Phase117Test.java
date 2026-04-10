import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase117Test {

    // ---- Map.remove(key, value) ----
    public static int testMapRemoveKeyValue() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1); m.put("b", 2);
        boolean removed = m.remove("a", 1);   // true
        boolean notRemoved = m.remove("b", 99); // false (value mismatch)
        return (removed ? 1 : 0) + (notRemoved ? 0 : 1) + m.size(); // 1+1+1 = 3
    }

    // ---- Collections.emptyList ----
    public static int testCollectionsEmptyList() {
        List<String> empty = Collections.emptyList();
        int a = empty.size(); // 0
        int threw = 0;
        try {
            empty.add("x");
        } catch (UnsupportedOperationException e) {
            threw = 10;
        }
        return a + threw; // 10
    }

    // ---- Stream.concat ----
    public static int testStreamConcat() {
        Stream<Integer> s1 = Stream.of(1, 2, 3);
        Stream<Integer> s2 = Stream.of(4, 5, 6);
        return Stream.concat(s1, s2).mapToInt(Integer::intValue).sum(); // 21
    }

    // ---- Iterable.forEach on Set ----
    public static int testSetForEach() {
        Set<Integer> set = new HashSet<>(Arrays.asList(1, 2, 3, 4, 5));
        int[] sum = {0};
        set.forEach(n -> sum[0] += n);
        return sum[0]; // 15
    }

    // ---- Map.replace ----
    public static int testMapReplace() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 10);
        m.replace("a", 99);
        Integer notExists = m.replace("b", 5); // returns null (key absent)
        return m.get("a") + (notExists == null ? 1 : 0); // 99 + 1 = 100
    }

    // ---- Integer.sum static ----
    public static int testIntegerSum() {
        return Integer.sum(100, 200) + Integer.sum(-50, 50); // 300 + 0 = 300
    }

    // ---- Stream.mapToObj ----
    public static int testIntStreamMapToObj() {
        return IntStream.range(1, 6)
            .mapToObj(Integer::toString)
            .collect(Collectors.joining(""))
            .length(); // "12345" = 5
    }

    // ---- Collections.unmodifiableSet ----
    public static int testUnmodifiableSet() {
        Set<String> mutable = new HashSet<>(Arrays.asList("a", "b", "c"));
        Set<String> readonly = Collections.unmodifiableSet(mutable);
        int size = readonly.size(); // 3
        int threw = 0;
        try {
            readonly.add("d");
        } catch (UnsupportedOperationException e) {
            threw = 10;
        }
        return size + threw; // 13
    }

    // ---- Optional.map ----
    public static int testOptionalMap() {
        int a = Optional.of("hello").map(String::length).orElse(0); // 5
        int b = Optional.<String>empty().map(String::length).orElse(-1); // -1
        return a + b; // 4
    }

    // ---- Array of arrays ----
    public static int testArrayOfArrays() {
        int[][] matrix = {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}};
        int sum = 0;
        for (int[] row : matrix) {
            for (int v : row) sum += v;
        }
        return sum; // 45
    }
}
