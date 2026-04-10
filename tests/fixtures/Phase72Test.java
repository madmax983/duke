import java.util.*;
import java.util.stream.*;

public class Phase72Test {

    // Stream.collect to List via Collectors.toList()
    public static int testStreamToList() {
        List<Integer> nums = new ArrayList<>();
        nums.add(3); nums.add(1); nums.add(4); nums.add(1); nums.add(5);
        List<Integer> sorted = nums.stream().sorted().collect(Collectors.toList());
        return sorted.get(0) * 10 + sorted.get(4); // 1*10 + 5 = 15
    }

    // Stream.filter then collect
    public static int testStreamFilterCollect() {
        List<Integer> nums = new ArrayList<>();
        for (int i = 1; i <= 10; i++) nums.add(i);
        List<Integer> evens = nums.stream()
            .filter(n -> n % 2 == 0)
            .collect(Collectors.toList());
        return evens.size(); // 5
    }

    // Stream.map then collect
    public static int testStreamMapCollect() {
        List<String> words = new ArrayList<>();
        words.add("hello"); words.add("world"); words.add("foo");
        List<Integer> lengths = words.stream()
            .map(String::length)
            .collect(Collectors.toList());
        int sum = 0;
        for (int len : lengths) sum += len;
        return sum; // 5+5+3 = 13
    }

    // IntStream.range
    public static int testIntStreamRange() {
        return IntStream.range(1, 6).sum(); // 1+2+3+4+5 = 15
    }

    // IntStream.rangeClosed
    public static int testIntStreamRangeClosed() {
        return IntStream.rangeClosed(1, 5).sum(); // 15
    }

    // Stream.count
    public static int testStreamCount() {
        List<String> words = new ArrayList<>();
        words.add("a"); words.add("bb"); words.add("ccc");
        long count = words.stream().filter(w -> w.length() > 1).count();
        return (int) count; // 2
    }

    // Stream.anyMatch
    public static int testStreamAnyMatch() {
        List<Integer> nums = new ArrayList<>();
        nums.add(1); nums.add(2); nums.add(3);
        int r = 0;
        if (nums.stream().anyMatch(n -> n > 2)) r += 1;
        if (!nums.stream().anyMatch(n -> n > 10)) r += 2;
        return r; // 3
    }

    // Stream.allMatch / noneMatch
    public static int testStreamMatchAll() {
        List<Integer> nums = new ArrayList<>();
        nums.add(2); nums.add(4); nums.add(6);
        int r = 0;
        if (nums.stream().allMatch(n -> n % 2 == 0)) r += 1;
        if (nums.stream().noneMatch(n -> n % 2 != 0)) r += 2;
        return r; // 3
    }
}
