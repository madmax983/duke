import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase57Test {

    // ---- Collectors.reducing (1-arg: BinaryOperator -> Optional) ----

    static int testCollectorsReducingNoIdentity() {
        Optional<Integer> result = Stream.of(
                Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3), Integer.valueOf(4))
            .collect(Collectors.reducing((a, b) -> Integer.valueOf(((Integer) a).intValue() + ((Integer) b).intValue())));
        return result.get().intValue();  // 1+2+3+4 = 10
    }

    static int testCollectorsReducingNoIdentityEmpty() {
        Optional<Integer> result = Stream.<Integer>of()
            .collect(Collectors.reducing((a, b) -> Integer.valueOf(((Integer) a).intValue() + ((Integer) b).intValue())));
        return result.isPresent() ? 1 : 0;  // 0 (empty)
    }

    // ---- Collectors.reducing (2-arg: identity + BinaryOperator -> T) ----

    static int testCollectorsReducingWithIdentity() {
        Integer result = (Integer) Stream.of(
                Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3), Integer.valueOf(4))
            .collect(Collectors.reducing(
                Integer.valueOf(0),
                (a, b) -> Integer.valueOf(((Integer) a).intValue() + ((Integer) b).intValue())
            ));
        return result.intValue();  // 0+1+2+3+4 = 10
    }

    static int testCollectorsReducingWithIdentityEmpty() {
        Integer result = (Integer) Stream.<Integer>of()
            .collect(Collectors.reducing(
                Integer.valueOf(42),
                (a, b) -> Integer.valueOf(((Integer) a).intValue() + ((Integer) b).intValue())
            ));
        return result.intValue();  // 42 (identity returned for empty stream)
    }

    // ---- Collectors.reducing (3-arg: identity + mapper + BinaryOperator -> U) ----

    static int testCollectorsReducingMapping() {
        // sum of lengths: "hi"=2, "hello"=5, "hey"=3 → total = 10
        Integer result = (Integer) Stream.of("hi", "hello", "hey")
            .collect(Collectors.reducing(
                Integer.valueOf(0),
                s -> Integer.valueOf(((String) s).length()),
                (a, b) -> Integer.valueOf(((Integer) a).intValue() + ((Integer) b).intValue())
            ));
        return result.intValue();  // 10
    }

    // ---- Stream.iterate(seed, predicate, next) ----

    static int testStreamIteratePredicate() {
        // 0, 1, 2, 3, 4 (while < 5)
        int sum = Stream.iterate(
                Integer.valueOf(0),
                n -> ((Integer) n).intValue() < 5,
                n -> Integer.valueOf(((Integer) n).intValue() + 1))
            .mapToInt(n -> ((Integer) n).intValue())
            .sum();
        return sum;  // 0+1+2+3+4 = 10
    }

    static int testStreamIteratePredicateEmpty() {
        // seed=10, predicate: n < 5 → immediately false → empty stream
        long count = Stream.iterate(
                Integer.valueOf(10),
                n -> ((Integer) n).intValue() < 5,
                n -> Integer.valueOf(((Integer) n).intValue() + 1))
            .count();
        return (int) count;  // 0
    }

    static int testStreamIteratePredicateCount() {
        // 1, 2, 4, 8, 16 (while <= 16, multiply by 2)
        long count = Stream.iterate(
                Integer.valueOf(1),
                n -> ((Integer) n).intValue() <= 16,
                n -> Integer.valueOf(((Integer) n).intValue() * 2))
            .count();
        return (int) count;  // 5
    }

    // ---- Optional.stream() ----

    static int testOptionalStreamPresent() {
        long count = Optional.of(Integer.valueOf(42)).stream().count();
        return (int) count;  // 1
    }

    static int testOptionalStreamEmpty() {
        long count = Optional.empty().stream().count();
        return (int) count;  // 0
    }

    // ---- ArrayDeque addFirst / addLast / peekFirst / peekLast ----

    static int testArrayDequeAddFirstPeekFirst() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        deque.addLast(Integer.valueOf(1));
        deque.addFirst(Integer.valueOf(2));
        // deque: [2, 1]
        return ((Integer) deque.peekFirst()).intValue();  // 2
    }

    static int testArrayDequeAddLastPeekLast() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        deque.addFirst(Integer.valueOf(1));
        deque.addLast(Integer.valueOf(2));
        // deque: [1, 2]
        return ((Integer) deque.peekLast()).intValue();  // 2
    }

    static int testArrayDequePollFirst() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        deque.addLast(Integer.valueOf(10));
        deque.addLast(Integer.valueOf(20));
        int first = ((Integer) deque.pollFirst()).intValue();
        return first + deque.size();  // 10 + 1 = 11
    }

    static int testArrayDequePollLast() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        deque.addLast(Integer.valueOf(10));
        deque.addLast(Integer.valueOf(20));
        int last = ((Integer) deque.pollLast()).intValue();
        return last + deque.size();  // 20 + 1 = 21
    }

    static int testArrayDequeContains() {
        ArrayDeque<String> deque = new ArrayDeque<>();
        deque.addLast("hello");
        deque.addLast("world");
        int a = deque.contains("hello") ? 1 : 0;
        int b = deque.contains("missing") ? 1 : 0;
        return a * 2 + b;  // 1*2 + 0 = 2
    }

    static int testArrayDequeClear() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        deque.addLast(Integer.valueOf(1));
        deque.addLast(Integer.valueOf(2));
        deque.addLast(Integer.valueOf(3));
        deque.clear();
        return deque.size();  // 0
    }

    static int testArrayDequeForEach() {
        ArrayDeque<Integer> deque = new ArrayDeque<>();
        deque.addLast(Integer.valueOf(1));
        deque.addLast(Integer.valueOf(2));
        deque.addLast(Integer.valueOf(3));
        int[] sum = {0};
        deque.forEach(n -> { sum[0] += ((Integer) n).intValue(); });
        return sum[0];  // 6
    }
}
