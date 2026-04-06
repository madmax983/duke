import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase116Test {

    // ---- TreeSet ordering ----
    public static int testTreeSet() {
        TreeSet<Integer> ts = new TreeSet<>();
        ts.add(5); ts.add(2); ts.add(8); ts.add(1); ts.add(9);
        int first = ts.first(); // 1
        int last = ts.last();   // 9
        return first + last + ts.size(); // 1 + 9 + 5 = 15
    }

    // ---- Stream.toArray ----
    public static int testStreamToArray() {
        Object[] arr = Stream.of("a", "bb", "ccc").toArray();
        return arr.length; // 3
    }

    // ---- Deque as queue ----
    public static int testDequeAsQueue() {
        Deque<Integer> queue = new ArrayDeque<>();
        queue.offer(1); queue.offer(2); queue.offer(3);
        int sum = 0;
        while (!queue.isEmpty()) {
            sum += queue.poll(); // FIFO: 1+2+3
        }
        return sum; // 6
    }

    // ---- Map.forEach accumulate ----
    public static int testMapForEachAccumulate() {
        Map<String, Integer> m = new TreeMap<>(); // sorted for determinism
        m.put("a", 1); m.put("b", 2); m.put("c", 3); m.put("d", 4);
        int[] product = {1};
        m.forEach((k, v) -> product[0] *= v);
        return product[0]; // 1*2*3*4 = 24
    }

    // ---- String.valueOf various ----
    public static int testStringValueOf() {
        String si = String.valueOf(42);
        String sd = String.valueOf(3.14);
        String sb = String.valueOf(true);
        return si.length() + sd.length() + sb.length(); // 2 + 4 + 4 = 10
    }

    // ---- IntStream.rangeClosed ----
    public static int testIntStreamRangeClosed() {
        return IntStream.rangeClosed(1, 5).sum(); // 1+2+3+4+5 = 15
    }

    // ---- Collectors.partitioningBy with downstream ----
    public static int testPartitioningByDownstream() {
        Map<Boolean, Long> result = Stream.of(1, 2, 3, 4, 5, 6)
            .collect(Collectors.partitioningBy(n -> n % 2 == 0, Collectors.counting()));
        return result.get(true).intValue() + result.get(false).intValue(); // 3 + 3 = 6
    }

    // ---- Optional.orElseGet ----
    public static int testOptionalOrElseGet() {
        int a = Optional.of(10).orElseGet(() -> 99); // 10
        int b = Optional.<Integer>empty().orElseGet(() -> 42); // 42
        return a + b; // 52
    }

    // ---- Stream.peek with count ----
    public static int testStreamPeekCount() {
        int[] sum = {0};
        long count = Stream.of(1, 2, 3, 4, 5)
            .peek(n -> sum[0] += n)
            .filter(n -> n % 2 == 0)
            .count(); // even: 2,4 → count=2, but peek runs on all 5
        return (int) count + sum[0]; // 2 + 15 = 17
    }

    // ---- Comparable interface ----
    static class Weight implements Comparable<Weight> {
        int kg;
        Weight(int kg) { this.kg = kg; }
        public int compareTo(Weight other) { return Integer.compare(this.kg, other.kg); }
    }
    public static int testComparable() {
        List<Weight> list = new ArrayList<>();
        list.add(new Weight(70)); list.add(new Weight(50)); list.add(new Weight(90));
        Collections.sort(list);
        return list.get(0).kg + list.get(2).kg; // 50 + 90 = 140
    }
}
