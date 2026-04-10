import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase104Test {

    // ---- Map.entrySet iteration ----
    public static int testMapEntrySetIteration() {
        Map<String, Integer> m = new LinkedHashMap<>();
        m.put("a", 1); m.put("b", 2); m.put("c", 3);
        int sum = 0;
        for (Map.Entry<String, Integer> e : m.entrySet()) {
            sum += e.getValue();
        }
        return sum; // 6
    }

    // ---- Stream.sorted() natural order ----
    public static int testStreamSortedNatural() {
        List<Integer> nums = Arrays.asList(5, 2, 8, 1, 9, 3);
        int first = nums.stream().sorted().findFirst().orElse(-1);
        return first; // 1
    }

    // ---- Optional.ofNullable ----
    public static int testOptionalOfNullable() {
        Optional<String> present = Optional.ofNullable("hello");
        Optional<String> empty = Optional.ofNullable(null);
        return present.map(String::length).orElse(0) + empty.map(String::length).orElse(0);
        // 5 + 0 = 5
    }

    // ---- List.of ----
    public static int testListOf() {
        List<Integer> list = List.of(1, 2, 3, 4, 5);
        int sum = 0;
        for (int v : list) sum += v;
        return sum; // 15
    }

    // ---- Map.of ----
    public static int testMapOf() {
        Map<String, Integer> m = Map.of("x", 10, "y", 20, "z", 30);
        return m.get("x") + m.get("y") + m.get("z"); // 60
    }

    // ---- Set.of ----
    public static int testSetOf() {
        Set<Integer> s = Set.of(1, 2, 3, 4, 5);
        return s.size(); // 5
    }

    // ---- String.valueOf(char) ----
    public static int testStringValueOfChar() {
        String s = String.valueOf('A');
        return s.length() + (s.equals("A") ? 10 : 0); // 1 + 10 = 11
    }

    // ---- Stream.collect(Collectors.joining) with stats ----
    public static int testStreamCollectJoining() {
        List<String> words = Arrays.asList("hello", "world");
        String joined = words.stream().collect(Collectors.joining(", "));
        return joined.length(); // "hello, world" = 12
    }

    // ---- Collections.swap ----
    public static int testCollectionsSwap() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        Collections.swap(list, 0, 4);
        return list.get(0) + list.get(4); // 5 + 1 = 6
    }

    // ---- Arrays.equals ----
    public static int testArraysEquals() {
        int[] a = {1, 2, 3};
        int[] b = {1, 2, 3};
        int[] c = {1, 2, 4};
        return (Arrays.equals(a, b) ? 1 : 0) + (Arrays.equals(a, c) ? 1 : 0); // 1 + 0 = 1
    }
}
