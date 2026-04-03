import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase54Test {

    // ---- IntStream.limit / skip ----

    static int testIntStreamLimit() {
        int sum = IntStream.of(1, 2, 3, 4, 5).limit(3).sum();
        return sum;  // 1+2+3 = 6
    }

    static int testIntStreamLimitZero() {
        long count = IntStream.of(1, 2, 3).limit(0).count();
        return (int) count;  // 0
    }

    static int testIntStreamSkip() {
        int sum = IntStream.of(1, 2, 3, 4, 5).skip(2).sum();
        return sum;  // 3+4+5 = 12
    }

    static int testIntStreamSkipAll() {
        long count = IntStream.of(1, 2, 3).skip(10).count();
        return (int) count;  // 0
    }

    static int testIntStreamLimitSkip() {
        // skip 1, take 3 → [2,3,4]
        int sum = IntStream.of(1, 2, 3, 4, 5).skip(1).limit(3).sum();
        return sum;  // 2+3+4 = 9
    }

    // ---- LongStream.limit / skip ----

    static int testLongStreamLimit() {
        long sum = LongStream.of(10L, 20L, 30L, 40L, 50L).limit(3).sum();
        return (int) sum;  // 10+20+30 = 60
    }

    static int testLongStreamSkip() {
        long sum = LongStream.of(10L, 20L, 30L, 40L, 50L).skip(3).sum();
        return (int) sum;  // 40+50 = 90
    }

    // ---- DoubleStream.limit / skip ----

    static int testDoubleStreamLimit() {
        double sum = DoubleStream.of(1.0, 2.0, 3.0, 4.0, 5.0).limit(3).sum();
        return (int) sum;  // 1+2+3 = 6
    }

    static int testDoubleStreamSkip() {
        double sum = DoubleStream.of(1.0, 2.0, 3.0, 4.0, 5.0).skip(2).sum();
        return (int) sum;  // 3+4+5 = 12
    }

    // ---- IntStream.flatMap ----

    static int testIntStreamFlatMap() {
        // flatMap each n to IntStream.of(n, n*10)
        int sum = IntStream.of(1, 2, 3)
            .flatMap(n -> IntStream.of(n, n * 10))
            .sum();
        return sum;  // (1+10) + (2+20) + (3+30) = 11+22+33 = 66
    }

    static int testIntStreamFlatMapRange() {
        // flatten ranges: [0,1] [0,1,2] [0,1,2,3]
        int count = (int) IntStream.of(2, 3, 4)
            .flatMap(n -> IntStream.range(0, n))
            .count();
        return count;  // 2+3+4 = 9
    }

    // ---- LongStream.flatMap ----

    static int testLongStreamFlatMap() {
        long sum = LongStream.of(1L, 2L, 3L)
            .flatMap(n -> LongStream.of(n, n * 10L))
            .sum();
        return (int) sum;  // 11+22+33 = 66
    }

    // ---- Collectors.mapping ----

    static int testCollectorsMapping() {
        // map String to length, collect to list
        List result = Stream.of("hi", "hello", "hey")
            .collect(Collectors.mapping(
                s -> Integer.valueOf(((String) s).length()),
                Collectors.toList()
            ));
        int sum = 0;
        for (int i = 0; i < result.size(); i++) {
            sum += ((Integer) result.get(i)).intValue();
        }
        return sum;  // 2+5+3 = 10
    }

    static int testCollectorsMappingJoining() {
        // map strings to uppercase, join
        String result = (String) Stream.of("a", "b", "c")
            .collect(Collectors.mapping(
                s -> ((String) s).toUpperCase(),
                Collectors.joining(", ")
            ));
        return result.equals("A, B, C") ? 1 : 0;  // 1
    }

    @SuppressWarnings("unchecked")
    static int testCollectorsMappingGrouped() {
        // group by length, then map each group's strings to uppercase and collect to list
        Collector<String, ?, List<String>> downstream =
            Collectors.mapping(s -> s.toUpperCase(), Collectors.toList());
        Collector<String, ?, Map<Integer, List<String>>> collector =
            Collectors.groupingBy(s -> Integer.valueOf(s.length()), downstream);
        Map<Integer, List<String>> grouped = Stream.of("a", "bb", "cc", "ddd")
            .collect(collector);
        // len 1 → ["A"], len 2 → ["BB","CC"], len 3 → ["DDD"]
        return grouped.get(Integer.valueOf(2)).size();  // 2
    }
}
