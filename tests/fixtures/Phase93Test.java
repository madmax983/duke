import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase93Test {

    // ---- Stack (Deque as stack) ----
    public static int testDequeAsStack() {
        Deque<Integer> stack = new ArrayDeque<>();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        int sum = 0;
        while (!stack.isEmpty()) {
            sum += stack.pop();
        }
        return sum; // 1+2+3 = 6
    }

    // ---- Queue (Deque as queue) ----
    public static int testDequeAsQueue() {
        Deque<Integer> queue = new ArrayDeque<>();
        queue.offer(10);
        queue.offer(20);
        queue.offer(30);
        return queue.poll() + queue.size(); // 10 + 2 = 12
    }

    // ---- Stream.peek (side effect inspection) ----
    public static int testStreamPeek() {
        int[] count = {0};
        int sum = IntStream.range(1, 6)
            .peek(n -> count[0]++)
            .sum();
        return sum + count[0]; // 15 + 5 = 20
    }

    // ---- Collectors.joining ----
    public static int testCollectorsJoining() {
        String result = Stream.of("a", "b", "c", "d")
            .collect(Collectors.joining(", ", "[", "]"));
        return result.length(); // "[a, b, c, d]" = 12
    }

    // ---- Map.computeIfAbsent ----
    public static int testComputeIfAbsent() {
        Map<String, List<Integer>> m = new HashMap<>();
        m.computeIfAbsent("nums", k -> new ArrayList<>()).add(1);
        m.computeIfAbsent("nums", k -> new ArrayList<>()).add(2);
        m.computeIfAbsent("nums", k -> new ArrayList<>()).add(3);
        return m.get("nums").size(); // 3
    }

    // ---- instanceof check ----
    public static int testInstanceOf() {
        Object o1 = "hello";
        Object o2 = 42;
        Object o3 = new ArrayList<>();
        int result = 0;
        if (o1 instanceof String) result += 1;
        if (o2 instanceof Integer) result += 10;
        if (o3 instanceof List) result += 100;
        return result; // 111
    }

    // ---- Conditional expression chains ----
    public static int testConditionalChain() {
        int n = 42;
        String cat = n < 10 ? "small" : n < 50 ? "medium" : "large";
        return cat.length(); // "medium" = 6
    }

    // ---- Array sort and binary search ----
    public static int testArraySortSearch() {
        int[] arr = {5, 3, 8, 1, 4, 2, 7, 6};
        Arrays.sort(arr);
        // arr is now [1,2,3,4,5,6,7,8]
        return arr[0] + arr[7]; // 1 + 8 = 9
    }

    // ---- String.join with List ----
    public static int testStringJoinList() {
        List<String> parts = Arrays.asList("Hello", "World", "Java");
        String joined = String.join("-", parts);
        return joined.length(); // "Hello-World-Java" = 16
    }

    // ---- Multiple return paths ----
    static int classify(int n) {
        if (n < 0) return -1;
        if (n == 0) return 0;
        if (n < 10) return 1;
        if (n < 100) return 2;
        return 3;
    }

    public static int testMultipleReturns() {
        return classify(-5) + classify(0) + classify(7) + classify(42) + classify(200);
        // -1 + 0 + 1 + 2 + 3 = 5
    }
}
