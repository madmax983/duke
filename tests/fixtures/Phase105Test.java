import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase105Test {

    // ---- Varargs method call ----
    static int sumAll(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }

    public static int testVarargs() {
        return sumAll(1, 2, 3, 4, 5); // 15
    }

    // ---- String.chars + mapToObj ----
    public static int testStringCharsMapToObj() {
        long count = "Hello World".chars()
            .mapToObj(c -> (char) c)
            .filter(c -> Character.isUpperCase(c))
            .count(); // H, W = 2
        return (int) count;
    }

    // ---- Collectors.toUnmodifiableList ----
    public static int testCollectorsToUnmodifiableList() {
        List<Integer> list = Stream.of(1, 2, 3, 4, 5)
            .collect(Collectors.toUnmodifiableList());
        int sum = list.stream().mapToInt(Integer::intValue).sum();
        int result = sum; // 15
        try {
            list.add(6);
        } catch (UnsupportedOperationException e) {
            result += 10; // 25
        }
        return result;
    }

    // ---- Map.compute ----
    public static int testMapComputeNew() {
        Map<String, Integer> m = new HashMap<>();
        m.compute("x", (k, v) -> v == null ? 1 : v + 10);
        m.compute("x", (k, v) -> v == null ? 1 : v + 10);
        m.compute("x", (k, v) -> v == null ? 1 : v + 10);
        return m.get("x"); // 1 -> 11 -> 21
    }

    // ---- Integer.compare ----
    public static int testIntegerCompare() {
        int a = Integer.compare(5, 10);  // negative
        int b = Integer.compare(10, 5);  // positive
        int c = Integer.compare(7, 7);   // 0
        // sign of each: a<0 → -1, b>0 → 1, c=0 → 0
        return (a < 0 ? 1 : 0) + (b > 0 ? 1 : 0) + (c == 0 ? 1 : 0); // 3
    }

    // ---- String.format %s with multiple args ----
    public static int testStringFormatMultipleArgs() {
        String s = String.format("%s has %d items", "list", 5);
        return s.length(); // "list has 5 items" = 16
    }

    // ---- Stream.collect(toMap) ----
    public static int testCollectToMap() {
        Map<String, Integer> m = Stream.of("a", "bb", "ccc")
            .collect(Collectors.toMap(s -> s, String::length));
        return m.get("a") + m.get("bb") + m.get("ccc"); // 1+2+3 = 6
    }

    // ---- OptionalInt operations ----
    public static int testOptionalInt() {
        OptionalInt present = OptionalInt.of(42);
        OptionalInt empty = OptionalInt.empty();
        return present.getAsInt() + empty.orElse(8); // 42 + 8 = 50
    }

    // ---- LongStream.range ----
    public static int testLongStreamRange() {
        long sum = LongStream.range(1, 6).sum(); // 1+2+3+4+5 = 15
        return (int) sum;
    }

    // ---- Collections.fill ----
    public static int testCollectionsFill() {
        List<String> list = new ArrayList<>(Arrays.asList("a", "b", "c", "d"));
        Collections.fill(list, "x");
        return list.stream().filter(s -> s.equals("x")).count() == 4 ? 1 : 0; // 1
    }
}
