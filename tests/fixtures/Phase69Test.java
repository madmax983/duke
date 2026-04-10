import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase69Test {

    // Collectors.toMap — key/value extractors
    public static int testCollectorsToMap() {
        List<String> words = new ArrayList<>();
        words.add("apple"); words.add("banana"); words.add("cherry");
        Map<Character, Integer> m = words.stream()
            .collect(Collectors.toMap(w -> w.charAt(0), String::length));
        return m.get('a') + m.get('b') + m.get('c'); // 5+6+6=17
    }

    // Stream.flatMap
    public static int testStreamFlatMap() {
        List<List<Integer>> nested = new ArrayList<>();
        List<Integer> a = new ArrayList<>(); a.add(1); a.add(2);
        List<Integer> b = new ArrayList<>(); b.add(3); b.add(4); b.add(5);
        nested.add(a); nested.add(b);
        return nested.stream()
            .flatMap(Collection::stream)
            .mapToInt(x -> x)
            .sum(); // 15
    }

    // Stream.peek (side-effect, but count is what matters)
    public static int testStreamPeek() {
        List<Integer> nums = new ArrayList<>();
        nums.add(1); nums.add(2); nums.add(3);
        int[] seen = {0};
        int result = nums.stream()
            .peek(x -> seen[0]++)
            .mapToInt(x -> x)
            .sum();
        return result + seen[0]; // 6 + 3 = 9
    }

    // Collectors.counting
    public static int testCollectorsCounting() {
        List<String> words = new ArrayList<>();
        words.add("a"); words.add("bb"); words.add("ccc"); words.add("dd");
        Map<Integer, Long> byLength = words.stream()
            .collect(Collectors.groupingBy(String::length, Collectors.counting()));
        return (int)(long)(byLength.get(1) + byLength.get(2) + byLength.get(3)); // 1+2+1=4
    }

    // OptionalInt/OptionalDouble operations
    public static int testOptionalInt() {
        List<Integer> nums = new ArrayList<>();
        nums.add(5); nums.add(10); nums.add(3);
        OptionalInt max = nums.stream().mapToInt(x -> x).max();
        return max.orElse(0); // 10
    }

    // String.chars() with collect
    public static int testStringCharsCount() {
        String s = "hello world";
        long vowels = s.chars()
            .filter(c -> "aeiou".indexOf(c) >= 0)
            .count();
        return (int) vowels; // e,o,o = 3
    }
}
