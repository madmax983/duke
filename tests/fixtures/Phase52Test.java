import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase52Test {

    // ---- Stream.flatMap ----

    static int testStreamFlatMap() {
        List<List> nested = new ArrayList<>();
        List<String> a = new ArrayList<>();
        a.add("hello");
        a.add("world");
        List<String> b = new ArrayList<>();
        b.add("foo");
        nested.add(a);
        nested.add(b);
        long count = nested.stream()
            .flatMap(l -> ((List) l).stream())
            .count();
        return (int) count;  // 3
    }

    static int testStreamFlatMapSum() {
        long sum = Stream.of("1,2,3", "4,5")
            .flatMap(s -> Arrays.stream(((String) s).split(",")))
            .mapToLong(s -> Long.parseLong((String) s))
            .sum();
        return (int) sum;  // 1+2+3+4+5 = 15
    }

    // ---- Arrays.stream ----

    static int testArraysStreamInt() {
        int[] arr = {1, 2, 3, 4, 5};
        int sum = Arrays.stream(arr).sum();
        return sum;  // 15
    }

    static int testArraysStreamRange() {
        int sum = Arrays.stream(new int[]{10, 20, 30, 40}, 1, 3).sum();
        return sum;  // 20+30 = 50
    }

    // ---- Optional.map ----

    static int testOptionalMap() {
        Optional<String> opt = Optional.of("hello");
        Optional mapped = opt.map(s -> Integer.valueOf(((String) s).length()));
        return ((Integer) mapped.get()).intValue();  // 5
    }

    static int testOptionalMapEmpty() {
        Optional<String> opt = Optional.empty();
        Optional mapped = opt.map(s -> Integer.valueOf(((String) s).length()));
        return mapped.isPresent() ? 1 : 0;  // 0
    }

    // ---- Optional.flatMap ----

    static int testOptionalFlatMap() {
        Optional<String> opt = Optional.of("42");
        Optional result = opt.flatMap(s -> Optional.of(Integer.valueOf(Integer.parseInt((String) s))));
        return ((Integer) result.get()).intValue();  // 42
    }

    // ---- Optional.orElse / orElseGet ----

    static int testOptionalOrElse() {
        Optional<Integer> empty = Optional.empty();
        return ((Integer) empty.orElse(Integer.valueOf(99))).intValue();  // 99
    }

    static int testOptionalOrElsePresent() {
        Optional<Integer> opt = Optional.of(Integer.valueOf(7));
        return ((Integer) opt.orElse(Integer.valueOf(99))).intValue();  // 7
    }

    static int testOptionalOrElseGet() {
        Optional<Integer> empty = Optional.empty();
        return ((Integer) empty.orElseGet(() -> Integer.valueOf(42))).intValue();  // 42
    }

    // ---- Optional.ifPresent ----

    static int testOptionalIfPresent() {
        int[] counter = {0};
        Optional<String> opt = Optional.of("hello");
        opt.ifPresent(s -> { counter[0] = ((String) s).length(); });
        return counter[0];  // 5
    }

    static int testOptionalIfPresentEmpty() {
        int[] counter = {0};
        Optional<String> opt = Optional.empty();
        opt.ifPresent(s -> { counter[0] = 1; });
        return counter[0];  // 0
    }

    // ---- Optional.filter ----

    static int testOptionalFilter() {
        Optional<Integer> opt = Optional.of(Integer.valueOf(10));
        Optional filtered = opt.filter(n -> ((Integer) n).intValue() > 5);
        return filtered.isPresent() ? ((Integer) filtered.get()).intValue() : 0;  // 10
    }

    static int testOptionalFilterOut() {
        Optional<Integer> opt = Optional.of(Integer.valueOf(3));
        Optional filtered = opt.filter(n -> ((Integer) n).intValue() > 5);
        return filtered.isPresent() ? 1 : 0;  // 0
    }

    // ---- Collectors.joining ----

    static int testCollectorsJoining() {
        List<String> words = Arrays.asList("a", "b", "c");
        String result = (String) words.stream().collect(Collectors.joining(", "));
        return result.equals("a, b, c") ? 1 : 0;  // 1
    }

    static int testCollectorsJoiningFull() {
        List<String> words = Arrays.asList("x", "y", "z");
        String result = (String) words.stream()
            .collect(Collectors.joining(", ", "[", "]"));
        return result.equals("[x, y, z]") ? 1 : 0;  // 1
    }

    // ---- Map.forEach ----

    static int testMapForEach() {
        Map<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(1));
        map.put("b", Integer.valueOf(2));
        map.put("c", Integer.valueOf(3));
        int[] sum = {0};
        map.forEach((k, v) -> { sum[0] += ((Integer) v).intValue(); });
        return sum[0];  // 6
    }

    // ---- Map.computeIfAbsent ----

    static int testMapComputeIfAbsent() {
        Map<String, List> map = new HashMap<>();
        List list = (List) map.computeIfAbsent("key", k -> new ArrayList());
        list.add("value");
        return ((List) map.get("key")).size();  // 1
    }

    // ---- Map.getOrDefault ----

    static int testMapGetOrDefault() {
        Map<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(5));
        int got = ((Integer) map.getOrDefault("a", Integer.valueOf(0))).intValue();
        int missing = ((Integer) map.getOrDefault("b", Integer.valueOf(-1))).intValue();
        return got + missing;  // 5 + (-1) = 4
    }
}
