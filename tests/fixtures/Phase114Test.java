import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase114Test {

    // ---- Stack class ----
    public static int testStackClass() {
        Stack<Integer> stack = new Stack<>();
        stack.push(10);
        stack.push(20);
        stack.push(30);
        int top = stack.peek(); // 30
        int popped = stack.pop(); // 30
        return top + popped + stack.size(); // 30 + 30 + 2 = 62
    }

    // ---- Collections.shuffle (fixed seed) ----
    public static int testCollectionsShuffle() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        Collections.shuffle(list, new Random(42));
        return list.stream().mapToInt(Integer::intValue).sum(); // always 15
    }

    // ---- Collectors.summingInt ----
    public static int testCollectorsSummingInt() {
        return Stream.of("a", "bb", "ccc", "dddd")
            .collect(Collectors.summingInt(String::length)); // 1+2+3+4 = 10
    }

    // ---- Integer.numberOfLeadingZeros ----
    public static int testIntegerLeadingZeros() {
        return Integer.numberOfLeadingZeros(1) + Integer.numberOfLeadingZeros(0x80000000);
        // 31 + 0 = 31
    }

    // ---- Map.values() iteration ----
    public static int testMapValuesIteration() {
        Map<String, Integer> m = new LinkedHashMap<>();
        m.put("a", 5); m.put("b", 10); m.put("c", 15);
        int sum = 0;
        for (int v : m.values()) sum += v;
        return sum; // 30
    }

    // ---- instanceof check ----
    public static int testInstanceOf() {
        Object s = "hello";
        Object n = 42;
        int a = (s instanceof String) ? 1 : 0;
        int b = (n instanceof Integer) ? 1 : 0;
        int c = (s instanceof Integer) ? 0 : 1;
        return a + b + c; // 3
    }

    // ---- String.substring edge cases ----
    public static int testStringSubstringEdge() {
        String s = "Hello, World!";
        String end = s.substring(7);      // "World!"
        String mid = s.substring(0, 5);   // "Hello"
        return end.length() + mid.length(); // 6 + 5 = 11
    }

    // ---- Stream.flatMap ----
    public static int testStreamFlatMap() {
        return Stream.of(
                Arrays.asList(1, 2, 3),
                Arrays.asList(4, 5),
                Arrays.asList(6, 7, 8, 9))
            .flatMap(Collection::stream)
            .mapToInt(Integer::intValue)
            .sum(); // 1+2+...+9 = 45
    }

    // ---- Enum values() + ordinal ----
    enum Season { SPRING, SUMMER, FALL, WINTER }
    public static int testEnumValues() {
        Season[] seasons = Season.values();
        int sum = 0;
        for (Season s : seasons) sum += s.ordinal();
        return sum; // 0+1+2+3 = 6
    }

    // ---- Integer.max / Integer.min static ----
    public static int testIntegerMaxMin() {
        int a = Integer.max(10, 20);
        int b = Integer.min(10, 20);
        return a + b; // 20 + 10 = 30
    }
}
