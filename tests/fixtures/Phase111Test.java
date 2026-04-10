import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase111Test {

    // ---- Map.putIfAbsent ----
    public static int testMapPutIfAbsent() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        m.putIfAbsent("a", 99); // key exists, no change
        m.putIfAbsent("b", 2);  // new key
        return m.get("a") + m.get("b"); // 1 + 2 = 3
    }

    // ---- Collections.reverse ----
    public static int testCollectionsReverse() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        Collections.reverse(list);
        return list.get(0) + list.get(4); // 5 + 1 = 6
    }

    // ---- Stream.sorted with Comparator ----
    public static int testStreamSortedComparator() {
        return Stream.of("banana", "apple", "cherry", "date")
            .sorted(Comparator.comparingInt(String::length))
            .mapToInt(String::length)
            .sum(); // 4+5+6+6 = 21
    }

    // ---- IntStream.range ----
    public static int testIntStreamRange() {
        return IntStream.range(1, 6).sum(); // 1+2+3+4+5 = 15
    }

    // ---- Collections.min/max ----
    public static int testCollectionsMinMax() {
        List<Integer> nums = Arrays.asList(3, 1, 4, 1, 5, 9, 2, 6);
        int min = Collections.min(nums);
        int max = Collections.max(nums);
        return min + max; // 1 + 9 = 10
    }

    // ---- String.contains ----
    public static int testStringContains() {
        String s = "Hello, World!";
        int a = s.contains("World") ? 1 : 0;
        int b = s.contains("Java") ? 0 : 1;
        return a + b; // 2
    }

    // ---- Collectors.toSet ----
    public static int testCollectorsToSet() {
        Set<Integer> set = Stream.of(1, 2, 2, 3, 3, 3)
            .collect(Collectors.toSet());
        return set.size(); // 3
    }

    // ---- Stream.anyMatch / allMatch / noneMatch ----
    public static int testStreamMatchers() {
        List<Integer> nums = Arrays.asList(2, 4, 6, 8);
        int a = nums.stream().allMatch(n -> n % 2 == 0) ? 1 : 0;
        int b = nums.stream().anyMatch(n -> n > 7) ? 1 : 0;
        int c = nums.stream().noneMatch(n -> n < 0) ? 1 : 0;
        return a + b + c; // 3
    }

    // ---- Varargs ----
    static int sum(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }
    public static int testVarargs() {
        return sum(1, 2, 3) + sum(10, 20); // 6 + 30 = 36
    }

    // ---- StringBuilder chaining ----
    public static int testStringBuilderChaining() {
        String result = new StringBuilder()
            .append("Hello")
            .append(", ")
            .append("World")
            .append("!")
            .toString();
        return result.length(); // 13
    }
}
