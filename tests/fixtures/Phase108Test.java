import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase108Test {

    // ---- String.substring edge cases ----
    public static int testSubstringEdge() {
        String s = "abcdefgh";
        String end = s.substring(5);          // "fgh" = 3
        String mid = s.substring(2, 6);       // "cdef" = 4
        String empty = s.substring(3, 3);     // "" = 0
        return end.length() + mid.length() + empty.length(); // 7
    }

    // ---- Arrays.asList mutations ----
    public static int testArraysAsListMutable() {
        // Arrays.asList returns a fixed-size list (set is ok, add throws)
        List<String> list = new ArrayList<>(Arrays.asList("a", "b", "c"));
        list.set(1, "B");
        return list.get(1).equals("B") ? list.size() : 0; // 3
    }

    // ---- Comparator.comparing with key extractor ----
    public static int testComparatorComparingChain() {
        List<String> words = new ArrayList<>(Arrays.asList("cat", "bee", "ant", "dog"));
        words.sort(Comparator.comparing(s -> s));  // alphabetical
        return words.get(0).length() + words.get(3).length(); // ant(3)+dog(3)=6
    }

    // ---- Map.forEach sum ----
    public static int testMapForEachSum() {
        Map<Integer, Integer> m = new HashMap<>();
        for (int i = 1; i <= 5; i++) m.put(i, i * i);
        int[] total = {0};
        m.forEach((k, v) -> total[0] += v);
        return total[0]; // 1+4+9+16+25 = 55
    }

    // ---- Stream.reduce with identity ----
    public static int testStreamReduceIdentity() {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        int product = nums.stream()
            .reduce(1, (a, b) -> a * b);
        return product; // 120
    }

    // ---- HashMap.compute edge: remove on null return ----
    public static int testHashMapComputeRemove() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 5);
        m.compute("a", (k, v) -> null);  // null return removes entry
        return m.containsKey("a") ? 1 : 0; // 0 (removed)
    }

    // ---- String[] sort ----
    public static int testStringArraySort() {
        String[] arr = {"cherry", "apple", "banana", "date"};
        Arrays.sort(arr);
        return arr[0].length() + arr[3].length(); // "apple"(5)+"date"(4)=9
    }

    // ---- Collectors.counting ----
    public static int testCollectorsCounting() {
        List<String> words = Arrays.asList("a", "b", "c", "d", "e");
        long count = words.stream().collect(Collectors.counting());
        return (int) count; // 5
    }

    // ---- BiFunction andThen ----
    public static int testBiFunctionAndThen() {
        BiFunction<Integer, Integer, Integer> add = (a, b) -> a + b;
        Function<Integer, String> toStr = n -> "Result:" + n;
        BiFunction<Integer, Integer, String> combined = add.andThen(toStr);
        String result = combined.apply(3, 7); // "Result:10"
        return result.length(); // 9
    }

    // ---- Interface default method ----
    interface Greeter {
        String greet(String name);
        default String greetLoud(String name) { return greet(name).toUpperCase(); }
    }

    public static int testInterfaceDefaultMethod() {
        Greeter g = name -> "Hello, " + name + "!";
        return g.greetLoud("world").length(); // "HELLO, WORLD!" = 13
    }
}
