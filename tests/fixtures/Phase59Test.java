import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase59Test {

    // ---- Stream.flatMapToInt ----

    static int testStreamFlatMapToInt() {
        // each string → IntStream of char codes, sum them
        int sum = Stream.of("ab", "cd")
            .flatMapToInt(s -> ((String) s).chars())
            .sum();
        // 'a'=97,'b'=98,'c'=99,'d'=100 → 394
        return sum;
    }

    static int testStreamFlatMapToIntCount() {
        long count = Stream.of("hi", "hello", "hey")
            .flatMapToInt(s -> ((String) s).chars())
            .count();
        return (int) count;  // 2+5+3 = 10
    }

    // ---- Stream.flatMapToLong ----

    static int testStreamFlatMapToLong() {
        long sum = Stream.of(Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3))
            .flatMapToLong(n -> LongStream.of((long) ((Integer) n).intValue(), (long) ((Integer) n).intValue() * 10L))
            .sum();
        return (int) sum;  // (1+10) + (2+20) + (3+30) = 66
    }

    // ---- Stream.flatMapToDouble ----

    static int testStreamFlatMapToDouble() {
        double sum = Stream.of(Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3))
            .flatMapToDouble(n -> DoubleStream.of((double) ((Integer) n).intValue()))
            .sum();
        return (int) sum;  // 1+2+3 = 6
    }

    // ---- Collectors.toMap (3-arg: keyFn, valFn, mergeFn) ----

    static int testCollectorsToMapMerge() {
        // Two elements map to same key "length", merge by summing values
        Map<Integer, Integer> m = Stream.of("hi", "ab", "hello")
            .collect(Collectors.toMap(
                s -> Integer.valueOf(((String) s).length()),
                s -> Integer.valueOf(1),
                (a, b) -> Integer.valueOf(((Integer) a).intValue() + ((Integer) b).intValue())
            ));
        // "hi"->1, "ab"->1 (merge: 2), "hello"->1
        // key=2: merge(1,1)=2; key=5: 1
        return ((Integer) m.get(Integer.valueOf(2))).intValue();  // 2
    }

    static int testCollectorsToMapMergeSize() {
        Map<Integer, String> m = Stream.of("a", "bb", "cc", "ddd")
            .collect(Collectors.toMap(
                s -> Integer.valueOf(((String) s).length()),
                s -> (String) s,
                (a, b) -> ((String) a) + "," + ((String) b)
            ));
        // key=1: "a", key=2: "bb,cc", key=3: "ddd"
        return m.size();  // 3
    }

    static int testCollectorsToMapMergeValue() {
        Map<Integer, String> m = Stream.of("a", "bb", "cc", "ddd")
            .collect(Collectors.toMap(
                s -> Integer.valueOf(((String) s).length()),
                s -> (String) s,
                (a, b) -> ((String) a) + "," + ((String) b)
            ));
        String merged = (String) m.get(Integer.valueOf(2));
        return merged.length();  // "bb,cc".length() = 5
    }

    // ---- HashSet.forEach ----

    static int testHashSetForEach() {
        Set<Integer> s = new HashSet<>();
        s.add(Integer.valueOf(1));
        s.add(Integer.valueOf(2));
        s.add(Integer.valueOf(3));
        int[] sum = {0};
        s.forEach(n -> { sum[0] += ((Integer) n).intValue(); });
        return sum[0];  // 6
    }

    // ---- TreeSet.forEach ----

    static int testTreeSetForEach() {
        TreeSet<String> s = new TreeSet<>();
        s.add("apple");
        s.add("banana");
        s.add("cherry");
        int[] count = {0};
        s.forEach(n -> { count[0]++; });
        return count[0];  // 3
    }

    // ---- TreeMap.forEach ----

    static int testTreeMapForEach() {
        TreeMap<String, Integer> m = new TreeMap<>();
        m.put("a", Integer.valueOf(1));
        m.put("b", Integer.valueOf(2));
        m.put("c", Integer.valueOf(3));
        int[] sum = {0};
        m.forEach((k, v) -> { sum[0] += ((Integer) v).intValue(); });
        return sum[0];  // 6
    }

    // ---- LinkedList.forEach ----

    static int testLinkedListForEach() {
        LinkedList<Integer> list = new LinkedList<>();
        list.add(Integer.valueOf(10));
        list.add(Integer.valueOf(20));
        list.add(Integer.valueOf(30));
        int[] sum = {0};
        list.forEach(n -> { sum[0] += ((Integer) n).intValue(); });
        return sum[0];  // 60
    }

    // ---- LinkedHashMap.forEach ----

    static int testLinkedHashMapForEach() {
        LinkedHashMap<String, Integer> m = new LinkedHashMap<>();
        m.put("x", Integer.valueOf(10));
        m.put("y", Integer.valueOf(20));
        m.put("z", Integer.valueOf(30));
        int[] sum = {0};
        m.forEach((k, v) -> { sum[0] += ((Integer) v).intValue(); });
        return sum[0];  // 60
    }

    // ---- PriorityQueue.forEach ----

    static int testPriorityQueueForEach() {
        PriorityQueue<Integer> pq = new PriorityQueue<>();
        pq.offer(Integer.valueOf(5));
        pq.offer(Integer.valueOf(3));
        pq.offer(Integer.valueOf(7));
        int[] sum = {0};
        pq.forEach(n -> { sum[0] += ((Integer) n).intValue(); });
        return sum[0];  // 15
    }
}
