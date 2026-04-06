import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase102Test {

    // ---- Map.putIfAbsent ----
    public static int testMapPutIfAbsent() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        m.putIfAbsent("a", 99);  // already exists, no change
        m.putIfAbsent("b", 2);   // new key inserted
        return m.get("a") + m.get("b"); // 1 + 2 = 3
    }

    // ---- Stream.sorted with Comparator ----
    public static int testStreamSortedComparator() {
        List<String> words = Arrays.asList("banana", "apple", "cherry", "date");
        String first = words.stream()
            .sorted(Comparator.comparingInt(String::length))
            .findFirst()
            .orElse("");
        return first.length(); // "date" = 4
    }

    // ---- IntStream.range sum ----
    public static int testIntStreamRangeSum() {
        return IntStream.range(1, 11).sum(); // 1+2+...+10 = 55
    }

    // ---- Collections.min/max ----
    public static int testCollectionsMinMax() {
        List<Integer> nums = Arrays.asList(3, 1, 4, 1, 5, 9, 2, 6);
        int min = Collections.min(nums);
        int max = Collections.max(nums);
        return min + max; // 1 + 9 = 10
    }

    // ---- List.indexOf / lastIndexOf ----
    public static int testListIndexOf() {
        List<String> list = Arrays.asList("a", "b", "c", "b", "a");
        int first = list.indexOf("b");     // 1
        int last = list.lastIndexOf("b");  // 3
        return first + last; // 4
    }

    // ---- StringBuilder.insert ----
    public static int testStringBuilderInsert() {
        StringBuilder sb = new StringBuilder("Hello World");
        sb.insert(5, ",");
        return sb.toString().length(); // "Hello, World" = 12
    }

    // ---- String.formatted (Java 15+, same as format) ----
    public static int testStringFormatted() {
        String s = "Value: %d".formatted(42);
        return s.length(); // "Value: 42" = 9
    }

    // ---- Map.containsValue ----
    public static int testMapContainsValue() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 10); m.put("b", 20); m.put("c", 30);
        int result = 0;
        if (m.containsValue(20)) result += 1;
        if (!m.containsValue(99)) result += 1;
        return result; // 2
    }

    // ---- Stream.noneMatch ----
    public static int testStreamNoneMatch() {
        List<Integer> nums = Arrays.asList(1, 3, 5, 7, 9);
        boolean hasEven = nums.stream().anyMatch(n -> n % 2 == 0);
        boolean allOdd = nums.stream().noneMatch(n -> n % 2 == 0);
        return (hasEven ? 0 : 1) + (allOdd ? 10 : 0); // 0 + 10 = 11
    }

    // ---- Arrays.sort(Object[]) ----
    public static int testArraysSortObjects() {
        String[] arr = {"banana", "apple", "cherry", "date"};
        Arrays.sort(arr);
        return arr[0].length() + arr[3].length(); // "apple"(5) + "date"(4) = 9
    }
}
