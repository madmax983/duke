import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase103Test {

    // ---- Stack ----
    public static int testStack() {
        Stack<Integer> stack = new Stack<>();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        int top = stack.peek();      // 3
        int popped = stack.pop();    // 3
        return top + popped + stack.size(); // 3 + 3 + 2 = 8
    }

    // ---- Collections.shuffle stability (size check) ----
    public static int testCollectionsShuffle() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        Collections.shuffle(list);
        return list.size(); // 5 (shuffle doesn't change size)
    }

    // ---- Stream.generate with limit ----
    public static int testStreamGenerate() {
        int[] counter = {0};
        return (int) Stream.generate(() -> { counter[0]++; return counter[0]; })
            .limit(5)
            .mapToInt(Integer::intValue)
            .sum(); // 1+2+3+4+5 = 15
    }

    // ---- String.join with list ----
    public static int testStringJoinList() {
        List<String> words = Arrays.asList("a", "b", "c", "d");
        String joined = String.join("-", words);
        return joined.length(); // "a-b-c-d" = 7
    }

    // ---- Map.getOrDefault ----
    public static int testMapGetOrDefault() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 10);
        int got = m.getOrDefault("a", 0);
        int missing = m.getOrDefault("z", 99);
        return got + missing; // 10 + 99 = 109
    }

    // ---- TreeSet ----
    public static int testTreeSet() {
        TreeSet<Integer> ts = new TreeSet<>();
        ts.add(5); ts.add(1); ts.add(3); ts.add(2); ts.add(4);
        return ts.first() + ts.last() + ts.size(); // 1 + 5 + 5 = 11
    }

    // ---- IntStream.rangeClosed ----
    public static int testIntStreamRangeClosed() {
        return IntStream.rangeClosed(1, 10).sum(); // 1+2+...+10 = 55
    }

    // ---- Comparator.reversed on method-ref ----
    public static int testComparatorReversedOnMethodRef() {
        List<String> words = new ArrayList<>(Arrays.asList("banana", "apple", "cherry", "date"));
        words.sort(Comparator.comparingInt(String::length).reversed());
        return words.get(0).length(); // longest = cherry = 6
    }

    // ---- Iterator remove ----
    public static int testIteratorRemove() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        Iterator<Integer> it = list.iterator();
        while (it.hasNext()) {
            if (it.next() % 2 == 0) it.remove(); // remove evens
        }
        return list.size(); // [1,3,5] = 3
    }

    // ---- Character.digit ----
    public static int testCharacterDigit() {
        int d3 = Character.digit('3', 10); // 3
        int da = Character.digit('a', 16); // 10
        return d3 + da; // 13
    }
}
