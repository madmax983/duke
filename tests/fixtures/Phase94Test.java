import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase94Test {

    // ---- Stream.collect(Collectors.counting()) ----
    public static int testCollectorsCounting() {
        List<String> words = Arrays.asList("apple", "banana", "cherry", "date", "elderberry");
        long count = words.stream()
            .filter(s -> s.length() > 5)
            .collect(Collectors.counting());
        return (int) count; // banana(6), cherry(6), elderberry(10) = 3
    }

    // ---- Stream.mapToObj ----
    public static int testIntStreamMapToObj() {
        String result = IntStream.range(1, 5)
            .mapToObj(Integer::toString)
            .collect(Collectors.joining("+"));
        return result.length(); // "1+2+3+4" = 7
    }

    // ---- Comparable interface ----
    static class Version implements Comparable<Version> {
        int major, minor;
        Version(int major, int minor) { this.major = major; this.minor = minor; }
        public int compareTo(Version other) {
            if (this.major != other.major) return this.major - other.major;
            return this.minor - other.minor;
        }
    }

    public static int testComparableImpl() {
        Version v1 = new Version(1, 5);
        Version v2 = new Version(2, 0);
        Version v3 = new Version(1, 8);
        List<Version> versions = new ArrayList<>(Arrays.asList(v2, v1, v3));
        Collections.sort(versions);
        return versions.get(0).major * 10 + versions.get(0).minor; // v1.0 = 15... wait v1=1.5, v3=1.8, v2=2.0
        // sorted: v1(1,5), v3(1,8), v2(2,0)
        // versions.get(0) = v1 → major=1, minor=5 → 1*10+5 = 15
    }

    // ---- Map.getOrDefault ----
    public static int testMapGetOrDefault() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 10);
        return m.getOrDefault("a", 0) + m.getOrDefault("b", 5); // 10 + 5 = 15
    }

    // ---- String.contains ----
    public static int testStringContains() {
        String s = "The quick brown fox";
        int result = 0;
        if (s.contains("quick")) result += 1;
        if (s.contains("slow")) result += 10;
        if (s.contains("fox")) result += 100;
        return result; // 101
    }

    // ---- int[][] 2D array ----
    public static int test2DArray() {
        int[][] matrix = {{1,2,3},{4,5,6},{7,8,9}};
        int sum = 0;
        for (int[] row : matrix) {
            for (int val : row) sum += val;
        }
        return sum; // 45
    }

    // ---- Static inner enum ----
    enum Direction { NORTH, SOUTH, EAST, WEST }

    public static int testEnumOrdinal() {
        return Direction.NORTH.ordinal() + Direction.SOUTH.ordinal()
             + Direction.EAST.ordinal() + Direction.WEST.ordinal(); // 0+1+2+3 = 6
    }

    // ---- while with complex condition ----
    public static int testWhileComplex() {
        int x = 100, steps = 0;
        while (x > 1) {
            if (x % 2 == 0) x /= 2;
            else x = x * 3 + 1;
            steps++;
        }
        return steps; // Collatz(100) = 25 steps
    }

    // ---- Map.remove ----
    public static int testMapRemove() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1); m.put("b", 2); m.put("c", 3);
        m.remove("b");
        return m.size() + m.getOrDefault("b", 0); // 2 + 0 = 2
    }

    // ---- Streams with boxed() and sum ----
    public static int testStreamBoxedSum() {
        List<Integer> list = Arrays.asList(3, 1, 4, 1, 5, 9, 2, 6);
        return list.stream()
            .mapToInt(Integer::intValue)
            .filter(n -> n >= 4)
            .sum(); // 4+5+9+6 = 24
    }
}
