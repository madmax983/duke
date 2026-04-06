import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase101Test {

    // ---- Comparator.thenComparing ----
    public static int testComparatorThenComparing() {
        List<String> words = new ArrayList<>(Arrays.asList("banana", "apple", "cherry", "avocado", "blueberry"));
        words.sort(Comparator.comparingInt(String::length).thenComparing(Comparator.naturalOrder()));
        // sorted by length, then alpha:
        // len 5: apple
        // len 6: banana, cherry
        // len 7: avocado
        // len 9: blueberry
        // alpha within same len: banana < cherry, so: apple, banana, cherry, avocado, blueberry
        return words.get(0).length() + words.get(1).length(); // 5 + 6 = 11
    }

    // ---- List.subList ----
    public static int testListSubList() {
        List<Integer> list = new ArrayList<>(Arrays.asList(10, 20, 30, 40, 50));
        List<Integer> sub = list.subList(1, 4); // [20, 30, 40]
        int sum = 0;
        for (int v : sub) sum += v;
        return sum; // 90
    }

    // ---- Collections.reverse ----
    public static int testCollectionsReverse() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        Collections.reverse(list);
        return list.get(0) + list.get(4); // 5 + 1 = 6
    }

    // ---- Collections.frequency ----
    public static int testCollectionsFrequency() {
        List<String> list = Arrays.asList("a", "b", "a", "c", "a", "b");
        return Collections.frequency(list, "a"); // 3
    }

    // ---- Map.merge ----
    public static int testMapMerge() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        m.merge("a", 5, Integer::sum);   // 1+5=6
        m.merge("b", 10, Integer::sum);  // new key = 10
        return m.get("a") + m.get("b"); // 6 + 10 = 16
    }

    // ---- Stream.distinct ----
    public static int testStreamDistinct() {
        List<Integer> nums = Arrays.asList(1, 2, 2, 3, 3, 3, 4);
        return (int) nums.stream().distinct().count(); // 4
    }

    // ---- Optional.filter ----
    public static int testOptionalFilter() {
        Optional<Integer> present = Optional.of(10);
        Optional<Integer> filtered = present.filter(n -> n > 5);
        Optional<Integer> empty = present.filter(n -> n > 100);
        return filtered.orElse(0) + empty.orElse(-1); // 10 + (-1) = 9
    }

    // ---- String.chars() stream ----
    public static int testStringCharsStream() {
        long count = "hello world".chars()
            .filter(c -> c == 'l')
            .count(); // 3
        return (int) count;
    }

    // ---- Arrays.stream(int[]) ----
    public static int testArraysStreamInt() {
        int[] arr = {1, 2, 3, 4, 5};
        return Arrays.stream(arr).filter(n -> n % 2 == 0).sum(); // 2+4=6
    }

    // ---- LinkedList as Deque ----
    public static int testLinkedListDeque() {
        LinkedList<Integer> deque = new LinkedList<>();
        deque.addFirst(1);
        deque.addLast(2);
        deque.addFirst(0);
        // [0, 1, 2]
        return deque.peekFirst() + deque.peekLast() + deque.size(); // 0+2+3=5
    }
}
