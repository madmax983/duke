import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase68Test {

    // mapToInt with instance method reference String::length
    public static int testMapToIntMethodRef() {
        List<String> words = new ArrayList<>();
        words.add("hi"); words.add("hello"); words.add("hey");
        return words.stream()
            .mapToInt(String::length)
            .sum(); // 2+5+3 = 10
    }

    // mapToInt with lambda
    public static int testMapToIntLambda() {
        List<Integer> nums = new ArrayList<>();
        nums.add(1); nums.add(2); nums.add(3);
        return nums.stream()
            .mapToInt(x -> x * x)
            .sum(); // 1+4+9 = 14
    }

    // Collectors.summingInt
    public static int testCollectorsSummingInt() {
        List<String> words = new ArrayList<>();
        words.add("abc"); words.add("de"); words.add("fghi");
        return words.stream()
            .collect(Collectors.summingInt(String::length)); // 3+2+4 = 9
    }

    // Stream.mapToLong
    public static int testMapToLong() {
        List<Integer> nums = new ArrayList<>();
        nums.add(1000000); nums.add(2000000); nums.add(3000000);
        long sum = nums.stream()
            .mapToLong(x -> (long) x * 2)
            .sum();
        return (int)(sum / 1000000); // 12
    }

    // Collectors.groupingBy
    public static int testGroupingBy() {
        List<String> words = new ArrayList<>();
        words.add("cat"); words.add("car"); words.add("dog"); words.add("dot");
        Map<Character, List<String>> grouped = words.stream()
            .collect(Collectors.groupingBy(w -> w.charAt(0)));
        return grouped.get('c').size() + grouped.get('d').size(); // 2+2=4
    }

    // Stream.distinct
    public static int testStreamDistinct() {
        List<Integer> nums = new ArrayList<>();
        nums.add(1); nums.add(2); nums.add(2); nums.add(3); nums.add(1);
        return (int) nums.stream().distinct().count(); // 3
    }

    // Stream.sorted
    public static int testStreamSorted() {
        List<Integer> nums = new ArrayList<>();
        nums.add(3); nums.add(1); nums.add(4); nums.add(1); nums.add(5);
        List<Integer> sorted = nums.stream()
            .sorted()
            .collect(Collectors.toList());
        return sorted.get(0) * 10 + sorted.get(4); // 1*10+5 = 15
    }

    // Stream.limit + skip
    public static int testStreamLimitSkip() {
        List<Integer> nums = new ArrayList<>();
        for (int i = 1; i <= 10; i++) nums.add(i);
        return nums.stream()
            .skip(2)
            .limit(3)
            .mapToInt(x -> x)
            .sum(); // 3+4+5 = 12
    }
}
