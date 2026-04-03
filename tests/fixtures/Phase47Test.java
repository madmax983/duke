import java.util.*;
import java.util.stream.*;

public class Phase47Test {

    // ---- List.of() ----

    static int testListOf() {
        List<String> list = List.of("a", "b", "c");
        return list.size();  // 3
    }

    static int testListOfGet() {
        List<Integer> list = List.of(Integer.valueOf(10), Integer.valueOf(20), Integer.valueOf(30));
        return ((Integer) list.get(1)).intValue();  // 20
    }

    // ---- Set.of() ----

    static int testSetOf() {
        Set<String> set = Set.of("x", "y", "z");
        return set.size();  // 3
    }

    static int testSetOfContains() {
        Set<String> set = Set.of("a", "b");
        return set.contains("a") ? 1 : 0;  // 1
    }

    // ---- Map.of() ----

    static int testMapOf() {
        Map<String, Integer> map = Map.of("a", Integer.valueOf(1), "b", Integer.valueOf(2));
        return map.size();  // 2
    }

    static int testMapOfGet() {
        Map<String, Integer> map = Map.of("x", Integer.valueOf(42));
        return ((Integer) map.get("x")).intValue();  // 42
    }

    // ---- Stream.flatMap ----

    static int testStreamFlatMap() {
        List<List<Integer>> nested = Arrays.asList(
            Arrays.asList(Integer.valueOf(1), Integer.valueOf(2)),
            Arrays.asList(Integer.valueOf(3), Integer.valueOf(4)),
            Arrays.asList(Integer.valueOf(5)));
        int sum = nested.stream()
            .flatMap(inner -> ((List<Integer>) inner).stream())
            .mapToInt(x -> ((Integer) x).intValue())
            .sum();
        return sum;  // 15
    }

    static int testStreamFlatMapSize() {
        List<String> words = Arrays.asList("hi", "hey");
        long count = words.stream()
            .flatMap(w -> Arrays.stream(((String) w).split("")))
            .count();
        return (int) count;  // 2+3 = 5
    }

    // ---- StringBuilder.delete / insert / reverse ----

    static int testStringBuilderDelete() {
        StringBuilder sb = new StringBuilder("hello");
        sb.delete(1, 3);  // removes "el" → "hlo"
        return sb.toString().equals("hlo") ? 1 : 0;  // 1
    }

    static int testStringBuilderInsert() {
        StringBuilder sb = new StringBuilder("hlo");
        sb.insert(1, "el");  // → "hello"
        return sb.toString().equals("hello") ? 1 : 0;  // 1
    }

    static int testStringBuilderReverse() {
        StringBuilder sb = new StringBuilder("abcd");
        sb.reverse();
        return sb.toString().equals("dcba") ? 1 : 0;  // 1
    }

    // ---- Collections.min / max ----

    static int testCollectionsMin() {
        List<Integer> list = Arrays.asList(
            Integer.valueOf(3), Integer.valueOf(1), Integer.valueOf(4), Integer.valueOf(2));
        return ((Integer) Collections.min(list)).intValue();  // 1
    }

    static int testCollectionsMax() {
        List<Integer> list = Arrays.asList(
            Integer.valueOf(3), Integer.valueOf(1), Integer.valueOf(4), Integer.valueOf(2));
        return ((Integer) Collections.max(list)).intValue();  // 4
    }
}
