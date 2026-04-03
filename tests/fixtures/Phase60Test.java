import java.util.*;
import java.util.stream.*;

public class Phase60Test {

    // ---- IntStream.takeWhile / dropWhile ----

    static int testIntStreamTakeWhile() {
        int sum = IntStream.of(1, 2, 3, 4, 5)
            .takeWhile(n -> n < 4)
            .sum();
        return sum;  // 1+2+3 = 6
    }

    static int testIntStreamDropWhile() {
        int sum = IntStream.of(1, 2, 3, 4, 5)
            .dropWhile(n -> n < 4)
            .sum();
        return sum;  // 4+5 = 9
    }

    // ---- LongStream.takeWhile / dropWhile ----

    static int testLongStreamTakeWhile() {
        long sum = LongStream.of(10L, 20L, 30L, 40L)
            .takeWhile(n -> n <= 20L)
            .sum();
        return (int) sum;  // 10+20 = 30
    }

    static int testLongStreamDropWhile() {
        long sum = LongStream.of(10L, 20L, 30L, 40L)
            .dropWhile(n -> n <= 20L)
            .sum();
        return (int) sum;  // 30+40 = 70
    }

    // ---- DoubleStream.takeWhile / dropWhile ----

    static int testDoubleStreamTakeWhile() {
        double sum = DoubleStream.of(1.5, 2.5, 3.5, 4.5)
            .takeWhile(n -> n < 3.0)
            .sum();
        return (int) sum;  // 1.5+2.5 = 4.0 → 4
    }

    static int testDoubleStreamDropWhile() {
        double sum = DoubleStream.of(1.5, 2.5, 3.5, 4.5)
            .dropWhile(n -> n < 3.0)
            .sum();
        return (int) sum;  // 3.5+4.5 = 8.0 → 8
    }

    // ---- Integer.compare / max / min ----

    static int testIntegerCompare() {
        return Integer.compare(5, 3);  // positive
    }

    static int testIntegerCompareEqual() {
        return Integer.compare(7, 7);  // 0
    }

    static int testIntegerMax() {
        return Integer.max(10, 20);  // 20
    }

    static int testIntegerMin() {
        return Integer.min(10, 20);  // 10
    }

    // ---- Long.compare / max / min ----

    static int testLongCompare() {
        long a = 100L, b = 50L;
        return Long.compare(a, b);  // positive
    }

    static int testLongMax() {
        return (int) Long.max(100L, 200L);  // 200
    }

    static int testLongMin() {
        return (int) Long.min(100L, 200L);  // 100
    }

    // ---- Double.compare / max / min ----

    static int testDoubleCompare() {
        return Double.compare(3.14, 2.71);  // positive
    }

    static int testDoubleMax() {
        return (int) Double.max(1.5, 2.5);  // 2
    }

    static int testDoubleMin() {
        return (int) Double.min(1.5, 2.5);  // 1
    }

    // ---- TreeMap.keySet / values / getOrDefault ----

    static int testTreeMapKeySet() {
        TreeMap<String, Integer> m = new TreeMap<>();
        m.put("a", Integer.valueOf(1));
        m.put("b", Integer.valueOf(2));
        m.put("c", Integer.valueOf(3));
        return m.keySet().size();  // 3
    }

    static int testTreeMapValues() {
        TreeMap<String, Integer> m = new TreeMap<>();
        m.put("x", Integer.valueOf(10));
        m.put("y", Integer.valueOf(20));
        int[] sum = {0};
        m.values().forEach(v -> { sum[0] += ((Integer) v).intValue(); });
        return sum[0];  // 30
    }

    static int testTreeMapGetOrDefault() {
        TreeMap<String, Integer> m = new TreeMap<>();
        m.put("key", Integer.valueOf(42));
        int present = ((Integer) m.getOrDefault("key", Integer.valueOf(0))).intValue();
        int absent = ((Integer) m.getOrDefault("missing", Integer.valueOf(99))).intValue();
        return present + absent;  // 42 + 99 = 141
    }

    // ---- TreeSet.stream ----

    static int testTreeSetStream() {
        TreeSet<String> s = new TreeSet<>();
        s.add("alpha");
        s.add("beta");
        s.add("gamma");
        long count = s.stream().count();
        return (int) count;  // 3
    }
}
