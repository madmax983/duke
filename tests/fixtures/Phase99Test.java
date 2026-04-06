import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase99Test {

    // ---- PriorityQueue ----
    public static int testPriorityQueue() {
        PriorityQueue<Integer> pq = new PriorityQueue<>();
        pq.add(5);
        pq.add(1);
        pq.add(3);
        pq.add(2);
        pq.add(4);
        int sum = 0;
        while (!pq.isEmpty()) {
            sum += pq.poll() * (pq.size() + 1); // weighted by remaining size+1
        }
        // poll order: 1,2,3,4,5
        // 1*(4+1)=5, 2*(3+1)=8, 3*(2+1)=9, 4*(1+1)=8, 5*(0+1)=5 = 35
        return sum; // 35
    }

    // ---- String.format with 0-padding ----
    public static int testStringFormatPadding() {
        String s = String.format("%05d", 42);
        return s.length(); // "00042" = 5
    }

    // ---- Chained method calls on result ----
    public static int testMethodChaining() {
        return "  Hello World  "
            .trim()
            .toLowerCase()
            .replace("world", "java")
            .length(); // "hello java" = 10
    }

    // ---- Iterable forEach ----
    public static int testIterableForEach() {
        List<Integer> list = Arrays.asList(1, 2, 3, 4, 5);
        int[] sum = {0};
        list.forEach(n -> sum[0] += n);
        return sum[0]; // 15
    }

    // ---- Map.entrySet stream ----
    public static int testMapEntrySetStream() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1); m.put("b", 2); m.put("c", 3);
        return m.entrySet().stream()
            .mapToInt(Map.Entry::getValue)
            .sum(); // 6
    }

    // ---- Arrays.copyOfRange ----
    public static int testArraysCopyOfRange() {
        int[] arr = {10, 20, 30, 40, 50};
        int[] slice = Arrays.copyOfRange(arr, 1, 4); // [20, 30, 40]
        int sum = 0;
        for (int v : slice) sum += v;
        return sum; // 90
    }

    // ---- Collectors.partitioningBy ----
    public static int testPartitioningBy() {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        Map<Boolean, List<Integer>> partitioned = nums.stream()
            .collect(Collectors.partitioningBy(n -> n % 2 == 0));
        return partitioned.get(true).size() + partitioned.get(false).size(); // 5 + 5 = 10
    }

    // ---- String.toCharArray and new String from char[] ----
    public static int testCharArrayConversion() {
        char[] chars = "Hello".toCharArray();
        chars[0] = 'J'; // Jello
        String result = new String(chars);
        return result.equals("Jello") ? result.length() : 0; // 5
    }

    // ---- Stream.min/max with Comparator ----
    public static int testStreamMinMax() {
        List<String> words = Arrays.asList("banana", "apple", "cherry", "date");
        String shortest = words.stream()
            .min(Comparator.comparingInt(String::length))
            .orElse("");
        String longest = words.stream()
            .max(Comparator.comparingInt(String::length))
            .orElse("");
        return shortest.length() + longest.length(); // 4 + 6 = 10
    }

    // ---- Integer.toBinaryString / toHexString ----
    public static int testIntegerRadixStrings() {
        String bin = Integer.toBinaryString(10);   // "1010"
        String hex = Integer.toHexString(255);     // "ff"
        return bin.length() + hex.length(); // 4 + 2 = 6
    }
}
