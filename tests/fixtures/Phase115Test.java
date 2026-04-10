import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase115Test {

    // ---- String.chars() with collect ----
    public static int testStringCharsToList() {
        List<Integer> chars = "abc".chars()
            .boxed()
            .collect(Collectors.toList());
        return chars.get(0) + chars.get(2); // 97 + 99 = 196
    }

    // ---- Optional.ifPresent ----
    public static int testOptionalIfPresent() {
        int[] result = {0};
        Optional.of(42).ifPresent(n -> result[0] = n);
        Optional.<Integer>empty().ifPresent(n -> result[0] = -1);
        return result[0]; // 42
    }

    // ---- Collections.singletonList ----
    public static int testSingletonList() {
        List<String> single = Collections.singletonList("hello");
        return single.size() + single.get(0).length(); // 1 + 5 = 6
    }

    // ---- Stream.reduce with identity ----
    public static int testStreamReduceIdentity() {
        int product = Stream.of(1, 2, 3, 4, 5)
            .reduce(1, (a, b) -> a * b);
        return product; // 120
    }

    // ---- Map.entrySet forEach ----
    public static int testMapEntrySetForEach() {
        Map<String, Integer> m = new HashMap<>();
        m.put("x", 10); m.put("y", 20); m.put("z", 30);
        int[] sum = {0};
        m.entrySet().forEach(e -> sum[0] += e.getValue());
        return sum[0]; // 60
    }

    // ---- Integer.reverse ----
    public static int testIntegerReverse() {
        // Integer.reverse(1) = 0x80000000 = -2147483648 in signed
        // but let's use something simple: Integer.reverse(0x12345678)
        // = Integer.reverse(305419896)
        // Java: Integer.reverse(1 << 31) = 1
        int r = Integer.reverse(1 << 31); // = 1
        return r; // 1
    }

    // ---- Collectors.toUnmodifiableMap ----
    public static int testCollectorsToUnmodifiableMap() {
        Map<String, Integer> m = Stream.of("a", "bb", "ccc")
            .collect(Collectors.toUnmodifiableMap(s -> s, String::length));
        int sum = m.get("a") + m.get("bb") + m.get("ccc"); // 1+2+3 = 6
        int threw = 0;
        try {
            m.put("d", 4);
        } catch (UnsupportedOperationException e) {
            threw = 10;
        }
        return sum + threw; // 16
    }

    // ---- Long.compare ----
    public static int testLongCompare() {
        int a = Long.compare(10L, 5L);   // positive
        int b = Long.compare(5L, 5L);    // 0
        int c = Long.compare(1L, 100L);  // negative
        return Integer.signum(a) + Integer.signum(b) + Integer.signum(c); // 1+0-1 = 0 ... let's use abs sum
        // Actually: signum(pos)=1, signum(0)=0, signum(neg)=-1 → sum=0
        // Make it return |a|+|b|+|c| via brute force: a>0?1:0 + b==0?1:0 + c<0?1:0 = 3
    }

    // ---- Arrays.stream ----
    public static int testArraysStream() {
        int[] arr = {5, 10, 15, 20};
        return Arrays.stream(arr).sum(); // 50
    }

    // ---- Nested generics ----
    public static int testNestedGenerics() {
        Map<String, List<Integer>> m = new HashMap<>();
        m.put("evens", Arrays.asList(2, 4, 6));
        m.put("odds", Arrays.asList(1, 3, 5));
        return m.get("evens").stream().mapToInt(Integer::intValue).sum()
             + m.get("odds").stream().mapToInt(Integer::intValue).sum(); // 12 + 9 = 21
    }
}
