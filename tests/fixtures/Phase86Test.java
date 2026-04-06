import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase86Test {

    // ---- Iterable for-each over custom class ----
    static class Range implements Iterable<Integer> {
        private final int start, end;
        Range(int start, int end) { this.start = start; this.end = end; }
        public Iterator<Integer> iterator() {
            return new Iterator<>() {
                int current = start;
                public boolean hasNext() { return current < end; }
                public Integer next() { return current++; }
            };
        }
    }

    public static int testCustomIterable() {
        int sum = 0;
        for (int n : new Range(1, 6)) sum += n; // 1+2+3+4+5 = 15
        return sum;
    }

    // ---- Comparable.compareTo on user class ----
    static class Box implements Comparable<Box> {
        final int value;
        Box(int v) { this.value = v; }
        public int compareTo(Box other) { return Integer.compare(this.value, other.value); }
    }

    public static int testComparable() {
        Box a = new Box(3), b = new Box(7);
        return a.compareTo(b) < 0 ? 1 : 0; // 3 < 7, so 1
    }

    // ---- Varargs ----
    static int sum(int... nums) {
        int total = 0;
        for (int n : nums) total += n;
        return total;
    }

    public static int testVarargs() {
        return sum(1, 2, 3, 4, 5); // 15
    }

    // ---- String.chars() stream ----
    public static int testStringChars() {
        return (int) "hello".chars().filter(c -> c == 'l').count(); // 2
    }

    // ---- TreeMap (sorted map) ----
    public static int testTreeMap() {
        TreeMap<String, Integer> map = new TreeMap<>();
        map.put("banana", 2);
        map.put("apple", 1);
        map.put("cherry", 3);
        // firstKey should be "apple"
        return map.get(map.firstKey()); // 1
    }

    // ---- LinkedList as Deque ----
    public static int testLinkedListDeque() {
        LinkedList<Integer> deque = new LinkedList<>();
        deque.addFirst(2);
        deque.addFirst(1);
        deque.addLast(3);
        return deque.removeFirst() + deque.removeLast(); // 1 + 3 = 4
    }

    // ---- Collections.frequency ----
    public static int testCollectionsFrequency() {
        List<String> list = new ArrayList<>(Arrays.asList("a", "b", "a", "c", "a"));
        return Collections.frequency(list, "a"); // 3
    }

    // ---- String.join ----
    public static int testStringJoin() {
        String result = String.join(", ", "one", "two", "three");
        return result.length(); // "one, two, three".length() = 15
    }

    // ---- Optional ----
    public static int testOptional() {
        Optional<String> opt = Optional.of("hello");
        return opt.isPresent() ? opt.get().length() : 0; // 5
    }

    // ---- Optional empty ----
    public static int testOptionalEmpty() {
        Optional<String> opt = Optional.empty();
        return opt.isPresent() ? 1 : 0; // 0
    }
}
