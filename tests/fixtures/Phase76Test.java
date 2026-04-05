import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase76Test {

    // Comparator.comparing
    public static int testComparatorComparing() {
        List<String> words = new ArrayList<>();
        words.add("banana"); words.add("apple"); words.add("cherry");
        words.sort(Comparator.comparing(String::length));
        // apple(5), banana(6), cherry(6) — first is shortest
        return words.get(0).length(); // 5
    }

    // Comparator.comparing reversed
    public static int testComparatorReversed() {
        List<String> words = new ArrayList<>();
        words.add("banana"); words.add("apple"); words.add("fig");
        words.sort(Comparator.comparing(String::length).reversed());
        return words.get(0).length(); // 6 (banana first)
    }

    // Comparator.naturalOrder
    public static int testComparatorNaturalOrder() {
        List<Integer> nums = new ArrayList<>();
        nums.add(5); nums.add(1); nums.add(3);
        nums.sort(Comparator.naturalOrder());
        return nums.get(0) * 10 + nums.get(2); // 1*10 + 5 = 15
    }

    // Comparator.reverseOrder
    public static int testComparatorReverseOrder() {
        List<Integer> nums = new ArrayList<>();
        nums.add(5); nums.add(1); nums.add(3);
        nums.sort(Comparator.reverseOrder());
        return nums.get(0) * 10 + nums.get(2); // 5*10 + 1 = 51
    }

    // Stream.sorted(Comparator)
    public static int testStreamSortedComparator() {
        List<String> words = new ArrayList<>();
        words.add("ccc"); words.add("a"); words.add("bb");
        List<String> sorted = words.stream()
            .sorted(Comparator.comparing(String::length))
            .collect(Collectors.toList());
        return sorted.get(0).length() * 10 + sorted.get(2).length(); // 1*10+3 = 13
    }

    // Map.values() stream
    public static int testMapValuesStream() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 10); m.put("b", 20); m.put("c", 30);
        int sum = m.values().stream().mapToInt(x -> x).sum();
        return sum; // 60
    }

    // Map.keySet() stream
    public static int testMapKeySetStream() {
        Map<String, Integer> m = new HashMap<>();
        m.put("hello", 1); m.put("world", 2);
        int totalLen = m.keySet().stream().mapToInt(String::length).sum();
        return totalLen; // 5+5 = 10
    }

    // Function.andThen composition
    public static int testFunctionCompose() {
        Function<Integer, Integer> times2 = x -> x * 2;
        Function<Integer, Integer> plus3 = x -> x + 3;
        Function<Integer, Integer> combined = times2.andThen(plus3);
        return combined.apply(5); // 5*2+3 = 13
    }
}
