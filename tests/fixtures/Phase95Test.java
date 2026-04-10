import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase95Test {

    // ---- Stream.generate with limit ----
    public static int testStreamGenerate() {
        int[] counter = {0};
        long count = Stream.generate(() -> { counter[0]++; return counter[0]; })
            .limit(5)
            .count();
        return (int) count + counter[0]; // 5 + 5 = 10
    }

    // ---- Optional.isPresent / ifPresent ----
    public static int testOptionalIsPresent() {
        Optional<String> present = Optional.of("hello");
        Optional<String> empty = Optional.empty();
        int result = 0;
        if (present.isPresent()) result += 1;
        if (!empty.isPresent()) result += 10;
        return result; // 11
    }

    // ---- Map.keySet iteration ----
    public static int testMapKeySet() {
        Map<String, Integer> m = new HashMap<>();
        m.put("one", 1); m.put("two", 2); m.put("three", 3);
        int sum = 0;
        for (String key : m.keySet()) {
            sum += m.get(key);
        }
        return sum; // 6
    }

    // ---- Arrays.stream(T[]) ----
    public static int testArraysStreamObj() {
        String[] words = {"hello", "world", "java"};
        return (int) Arrays.stream(words)
            .filter(s -> s.length() == 5)
            .count(); // hello, world = 2
    }

    // ---- Math operations ----
    public static int testMathOps() {
        int a = Math.max(10, 20);   // 20
        int b = Math.min(10, 20);   // 10
        int c = Math.abs(-15);      // 15
        return a + b + c; // 45
    }

    // ---- StringBuilder chain ----
    public static int testStringBuilderChain() {
        String result = new StringBuilder()
            .append("Hello")
            .append(", ")
            .append("World")
            .append("!")
            .toString();
        return result.length(); // 13
    }

    // ---- try-finally return value ----
    public static int testTryFinallyReturn() {
        try {
            return 42;
        } finally {
            // finally runs but doesn't change return value
            int x = 1 + 1;
        }
    }

    // ---- Nested HashMap ----
    public static int testNestedMap() {
        Map<String, Map<String, Integer>> outer = new HashMap<>();
        Map<String, Integer> inner = new HashMap<>();
        inner.put("x", 10);
        inner.put("y", 20);
        outer.put("coords", inner);
        return outer.get("coords").get("x") + outer.get("coords").get("y"); // 30
    }

    // ---- Stream.anyMatch / allMatch / noneMatch ----
    public static int testStreamMatching() {
        List<Integer> nums = Arrays.asList(2, 4, 6, 8, 10);
        int result = 0;
        if (nums.stream().allMatch(n -> n % 2 == 0)) result += 1;
        if (nums.stream().anyMatch(n -> n > 8)) result += 10;
        if (nums.stream().noneMatch(n -> n < 0)) result += 100;
        return result; // 111
    }

    // ---- String.replaceAll with regex ----
    public static int testStringReplaceAll() {
        String s = "Hello   World   Java";
        String cleaned = s.replaceAll("\\s+", " ");
        return cleaned.length(); // "Hello World Java" = 16
    }
}
