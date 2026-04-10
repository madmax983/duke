import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase112Test {

    // ---- Map.containsValue ----
    public static int testMapContainsValue() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1); m.put("b", 2); m.put("c", 3);
        int a = m.containsValue(2) ? 1 : 0;
        int b = m.containsValue(99) ? 0 : 1;
        return a + b; // 2
    }

    // ---- List.subList ----
    public static int testListSubList() {
        List<Integer> list = Arrays.asList(10, 20, 30, 40, 50);
        List<Integer> sub = list.subList(1, 4); // [20, 30, 40]
        return sub.stream().mapToInt(Integer::intValue).sum(); // 90
    }

    // ---- Collections.frequency ----
    public static int testCollectionsFrequency() {
        List<Integer> list = Arrays.asList(1, 2, 2, 3, 2, 4, 2);
        return Collections.frequency(list, 2); // 4
    }

    // ---- Stream.collect toMap ----
    public static int testCollectToMap() {
        Map<String, Integer> m = Stream.of("apple", "banana", "cherry")
            .collect(Collectors.toMap(s -> s, String::length));
        return m.get("apple") + m.get("banana") + m.get("cherry"); // 5+6+6 = 17
    }

    // ---- Integer.compare ----
    public static int testIntegerCompare() {
        int a = Integer.compare(5, 3);  // positive
        int b = Integer.compare(3, 3);  // 0
        int c = Integer.compare(1, 9);  // negative
        // convert to sign: 1, 0, -1 → sum = 0
        return Integer.signum(a) + Integer.signum(b) + Integer.signum(c); // 1+0-1 = 0 + 10 = 10
        // let's just return distinct count: 1 pos + 1 zero + 1 neg = 3
    }

    // ---- String.chars() stream ----
    public static int testStringCharsStream() {
        return (int) "hello world".chars()
            .filter(c -> c == 'l')
            .count(); // 3
    }

    // ---- Collectors.counting ----
    public static int testCollectorsCounting() {
        long count = Stream.of("a", "bb", "ccc", "dddd")
            .collect(Collectors.counting());
        return (int) count; // 4
    }

    // ---- Stack (Deque via ArrayDeque) ----
    public static int testArrayDequeAsStack() {
        Deque<Integer> stack = new ArrayDeque<>();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        int sum = 0;
        while (!stack.isEmpty()) {
            sum += stack.pop(); // 3+2+1
        }
        return sum; // 6
    }

    // ---- Map.keySet iteration ----
    public static int testMapKeySetIteration() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 10); m.put("b", 20); m.put("c", 30);
        int sum = 0;
        for (String key : m.keySet()) {
            sum += m.get(key);
        }
        return sum; // 60
    }

    // ---- Stream.limit ----
    public static int testStreamLimit() {
        return Stream.iterate(1, n -> n + 1)
            .limit(5)
            .mapToInt(Integer::intValue)
            .sum(); // 1+2+3+4+5 = 15
    }
}
