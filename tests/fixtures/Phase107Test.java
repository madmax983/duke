import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase107Test {

    // ---- Enum with methods ----
    enum Season {
        SPRING, SUMMER, FALL, WINTER;
        public boolean isWarm() { return this == SPRING || this == SUMMER; }
    }

    public static int testEnumWithMethods() {
        int count = 0;
        for (Season s : Season.values()) {
            if (s.isWarm()) count++;
        }
        return count; // 2
    }

    // ---- instanceof pattern ----
    public static int testInstanceof() {
        Object a = "hello";
        Object b = 42;
        Object c = 3.14;
        int result = 0;
        if (a instanceof String) result += 1;
        if (b instanceof Integer) result += 1;
        if (c instanceof Double) result += 1;
        return result; // 3
    }

    // ---- Ternary chains ----
    public static int testTernaryChain() {
        int x = 15;
        String s = x < 10 ? "small" : x < 20 ? "medium" : "large";
        return s.length(); // "medium" = 6
    }

    // ---- Static initializer block ----
    static class Counter {
        static int count;
        static {
            count = 10;
        }
        static void increment() { count++; }
    }

    public static int testStaticInitializer() {
        Counter.increment();
        Counter.increment();
        Counter.increment();
        return Counter.count; // 13
    }

    // ---- String.chars().distinct().count() ----
    public static int testStringCharsDistinct() {
        long distinct = "mississippi".chars().distinct().count();
        return (int) distinct; // m,i,s,p = 4
    }

    // ---- Map.computeIfAbsent ----
    public static int testMapComputeIfAbsent() {
        Map<String, List<Integer>> m = new HashMap<>();
        m.computeIfAbsent("a", k -> new ArrayList<>()).add(1);
        m.computeIfAbsent("a", k -> new ArrayList<>()).add(2);
        m.computeIfAbsent("b", k -> new ArrayList<>()).add(3);
        return m.get("a").size() + m.get("b").size(); // 2 + 1 = 3
    }

    // ---- Collections.singletonList ----
    public static int testSingletonList() {
        List<String> list = Collections.singletonList("hello");
        return list.size() + list.get(0).length(); // 1 + 5 = 6
    }

    // ---- Stream.count after filter ----
    public static int testStreamCount() {
        long count = IntStream.range(0, 100)
            .filter(n -> n % 3 == 0 || n % 5 == 0)
            .count();
        return (int) count; // 47 (multiples of 3 or 5 under 100)
    }

    // ---- StringBuilder chaining ----
    public static int testStringBuilderChain() {
        String result = new StringBuilder()
            .append("Hello")
            .append(", ")
            .append("World")
            .append("!")
            .toString();
        return result.length(); // 13
    }

    // ---- Comparable interface ----
    static class Weight implements Comparable<Weight> {
        int kg;
        Weight(int kg) { this.kg = kg; }
        public int compareTo(Weight o) { return Integer.compare(this.kg, o.kg); }
    }

    public static int testComparable() {
        List<Weight> weights = new ArrayList<>();
        weights.add(new Weight(5));
        weights.add(new Weight(2));
        weights.add(new Weight(8));
        Collections.sort(weights);
        return weights.get(0).kg + weights.get(2).kg; // 2 + 8 = 10
    }
}
