import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase110Test {

    // ---- String.join ----
    public static int testStringJoin() {
        String joined = String.join(", ", "a", "b", "c");
        return joined.length(); // "a, b, c" = 7
    }

    // ---- String.join with List ----
    public static int testStringJoinList() {
        List<String> parts = Arrays.asList("foo", "bar", "baz");
        String joined = String.join("-", parts);
        return joined.length(); // "foo-bar-baz" = 11
    }

    // ---- Collectors.joining ----
    public static int testCollectorsJoining() {
        String result = Stream.of("a", "b", "c", "d")
            .collect(Collectors.joining(", ", "[", "]"));
        return result.length(); // "[a, b, c, d]" = 12
    }

    // ---- Map.getOrDefault ----
    public static int testMapGetOrDefault() {
        Map<String, Integer> m = new HashMap<>();
        m.put("x", 42);
        int got = m.getOrDefault("x", 0);
        int missing = m.getOrDefault("y", 99);
        return got + missing; // 42 + 99 = 141
    }

    // ---- Optional.filter ----
    public static int testOptionalFilter() {
        Optional<Integer> opt = Optional.of(10);
        int present = opt.filter(n -> n > 5).map(n -> n * 2).orElse(0); // 20
        int filtered = opt.filter(n -> n > 20).orElse(-1); // -1
        return present + filtered; // 20 + (-1) = 19
    }

    // ---- Stream.distinct ----
    public static int testStreamDistinct() {
        return (int) Stream.of(1, 2, 2, 3, 3, 3, 4)
            .distinct()
            .count(); // 4
    }

    // ---- Collections.sort with Comparator ----
    public static int testCollectionsSortComparator() {
        List<String> words = new ArrayList<>(Arrays.asList("banana", "apple", "cherry", "date"));
        Collections.sort(words, Comparator.comparingInt(String::length));
        return words.get(0).length() + words.get(3).length(); // "date"=4 + "banana"/"cherry"=6 → 4+6=10
    }

    // ---- Iterator pattern ----
    public static int testIteratorPattern() {
        List<Integer> list = Arrays.asList(1, 2, 3, 4, 5);
        Iterator<Integer> it = list.iterator();
        int sum = 0;
        while (it.hasNext()) {
            sum += it.next();
        }
        return sum; // 15
    }

    // ---- Map.merge ----
    public static int testMapMerge() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        m.merge("a", 10, Integer::sum); // 1+10 = 11
        m.merge("b", 5, Integer::sum);  // new key = 5
        return m.get("a") + m.get("b"); // 11 + 5 = 16
    }

    // ---- Ternary in stream ----
    public static int testTernaryInStream() {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6);
        return nums.stream()
            .mapToInt(n -> n % 2 == 0 ? n * 10 : n)
            .sum(); // 1 + 20 + 3 + 40 + 5 + 60 = 129
    }
}
