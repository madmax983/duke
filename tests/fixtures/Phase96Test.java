import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase96Test {

    // ---- Recursive data structure (linked list) ----
    static class Node {
        int val;
        Node next;
        Node(int val) { this.val = val; }
        Node(int val, Node next) { this.val = val; this.next = next; }
    }

    static int listSum(Node n) {
        if (n == null) return 0;
        return n.val + listSum(n.next);
    }

    public static int testLinkedListRecursion() {
        Node list = new Node(1, new Node(2, new Node(3, new Node(4, new Node(5, null)))));
        return listSum(list); // 15
    }

    // ---- Generic method ----
    static <T extends Comparable<T>> T max(T a, T b) {
        return a.compareTo(b) >= 0 ? a : b;
    }

    public static int testGenericMethod() {
        int maxInt = max(10, 20);  // 20
        String maxStr = max("apple", "banana"); // "banana"
        return maxInt + maxStr.length(); // 20 + 6 = 26
    }

    // ---- Exception with cause ----
    public static int testExceptionCause() {
        try {
            try {
                throw new RuntimeException("cause");
            } catch (RuntimeException e) {
                throw new IllegalStateException("wrapper", e);
            }
        } catch (IllegalStateException e) {
            return e.getMessage().length() + e.getCause().getMessage().length();
            // "wrapper".length() + "cause".length() = 7 + 5 = 12
        }
    }

    // ---- Bitwise operations ----
    public static int testBitwiseOps() {
        int a = 0b1010_1010; // 170
        int b = 0b1100_1100; // 204
        int andResult = a & b;   // 0b1000_1000 = 136
        int orResult = a | b;    // 0b1110_1110 = 238
        int xorResult = a ^ b;   // 0b0110_0110 = 102
        return (andResult + orResult + xorResult) % 100; // (136+238+102) % 100 = 476 % 100 = 76
    }

    // ---- Shift operations ----
    public static int testShiftOps() {
        int x = 8;
        return (x << 2) + (x >> 1); // 32 + 4 = 36
    }

    // ---- Stream.collect to list then sort ----
    public static int testCollectThenSort() {
        List<Integer> sorted = Stream.of(5, 3, 8, 1, 4)
            .collect(Collectors.toList());
        Collections.sort(sorted);
        return sorted.get(0) + sorted.get(4); // 1 + 8 = 9
    }

    // ---- String.matches ----
    public static int testStringMatches() {
        String[] values = {"123", "abc", "12a", "456"};
        int count = 0;
        for (String s : values) {
            if (s.matches("\\d+")) count++;
        }
        return count; // "123" and "456" = 2
    }

    // ---- Multidimensional array sum ----
    public static int testMultiDimSum() {
        int[][] grid = new int[3][3];
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                grid[i][j] = i * 3 + j + 1; // 1..9
        int sum = 0;
        for (int[] row : grid)
            for (int v : row)
                sum += v;
        return sum; // 45
    }

    // ---- instanceof with cast ----
    static int processObj(Object o) {
        if (o instanceof String) return ((String) o).length();
        if (o instanceof Integer) return ((Integer) o) * 2;
        if (o instanceof List) return ((List<?>) o).size();
        return -1;
    }

    public static int testInstanceOfCast() {
        return processObj("hello") + processObj(21) + processObj(Arrays.asList(1,2,3));
        // 5 + 42 + 3 = 50
    }

    // ---- HashMap with int values accumulation ----
    public static int testMapAccumulation() {
        String text = "hello world hello java world";
        Map<String, Integer> freq = new HashMap<>();
        for (String word : text.split(" ")) {
            freq.put(word, freq.getOrDefault(word, 0) + 1);
        }
        return freq.get("hello") + freq.get("world") + freq.get("java");
        // 2 + 2 + 1 = 5
    }
}
