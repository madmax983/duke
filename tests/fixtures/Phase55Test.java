import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase55Test {

    // ---- DoubleStream.forEach ----

    static int testDoubleStreamForEach() {
        int[] sum = {0};
        DoubleStream.of(1.0, 2.0, 3.0).forEach(d -> { sum[0] += (int) d; });
        return sum[0];  // 6
    }

    // ---- DoubleStream.anyMatch / allMatch / noneMatch ----

    static int testDoubleStreamAnyMatch() {
        boolean result = DoubleStream.of(1.0, 2.0, 3.5)
            .anyMatch(d -> d > 3.0);
        return result ? 1 : 0;  // 1
    }

    static int testDoubleStreamAllMatch() {
        boolean result = DoubleStream.of(2.0, 4.0, 6.0)
            .allMatch(d -> d % 2.0 == 0.0);
        return result ? 1 : 0;  // 1
    }

    static int testDoubleStreamNoneMatch() {
        boolean result = DoubleStream.of(1.1, 2.2, 3.3)
            .noneMatch(d -> d > 10.0);
        return result ? 1 : 0;  // 1
    }

    // ---- DoubleStream.findFirst ----

    static int testDoubleStreamFindFirst() {
        OptionalDouble result = DoubleStream.of(10.0, 20.0, 30.0).findFirst();
        return (int) result.getAsDouble();  // 10
    }

    static int testDoubleStreamFindFirstEmpty() {
        OptionalDouble result = DoubleStream.of().findFirst();
        return result.isPresent() ? 1 : 0;  // 0
    }

    // ---- DoubleStream.reduce ----

    static int testDoubleStreamReduceIdentity() {
        double result = DoubleStream.of(1.0, 2.0, 3.0, 4.0)
            .reduce(0.0, (a, b) -> a + b);
        return (int) result;  // 10
    }

    static int testDoubleStreamReduceOptional() {
        OptionalDouble result = DoubleStream.of(2.0, 3.0, 4.0)
            .reduce((a, b) -> a * b);
        return (int) result.getAsDouble();  // 24
    }

    // ---- DoubleStream.flatMap ----

    static int testDoubleStreamFlatMap() {
        double sum = DoubleStream.of(1.0, 2.0, 3.0)
            .flatMap(d -> DoubleStream.of(d, d * 10.0))
            .sum();
        return (int) sum;  // (1+10) + (2+20) + (3+30) = 66
    }

    // ---- DoubleStream.mapToInt / mapToLong ----

    static int testDoubleStreamMapToInt() {
        int sum = DoubleStream.of(1.7, 2.3, 3.9)
            .mapToInt(d -> (int) d)
            .sum();
        return sum;  // 1+2+3 = 6
    }

    static int testDoubleStreamMapToLong() {
        long sum = DoubleStream.of(100.0, 200.0, 300.0)
            .mapToLong(d -> (long) d)
            .sum();
        return (int) sum;  // 600
    }

    // ---- DoubleStream.distinct ----

    static int testDoubleStreamDistinct() {
        long count = DoubleStream.of(1.0, 2.0, 1.0, 3.0, 2.0)
            .distinct()
            .count();
        return (int) count;  // 3
    }

    // ---- DoubleStream.boxed ----

    static int testDoubleStreamBoxed() {
        long count = DoubleStream.of(1.0, 2.0, 3.0)
            .boxed()
            .count();
        return (int) count;  // 3
    }

    // ---- LongStream.reduce (optional) ----

    static int testLongStreamReduceOptional() {
        OptionalLong result = LongStream.of(2L, 3L, 4L)
            .reduce((a, b) -> a * b);
        return (int) result.getAsLong();  // 24
    }

    static int testLongStreamReduceOptionalEmpty() {
        OptionalLong result = LongStream.of().reduce((a, b) -> a + b);
        return result.isPresent() ? 1 : 0;  // 0
    }

    // ---- LongStream.mapToDouble ----

    static int testLongStreamMapToDouble() {
        double sum = LongStream.of(1L, 2L, 3L)
            .mapToDouble(n -> (double) n * 1.5)
            .sum();
        return (int) sum;  // (1.5+3.0+4.5) = 9
    }

    // ---- Collectors.summingLong ----

    static int testCollectorsSummingLong() {
        List<String> words = Arrays.asList("hi", "hello", "hey");
        long sum = (Long) words.stream()
            .collect(Collectors.summingLong(s -> (long) ((String) s).length()));
        return (int) sum;  // 2+5+3 = 10
    }

    // ---- Collectors.averagingDouble ----

    static int testCollectorsAveragingDouble() {
        List<String> words = Arrays.asList("hi", "hello", "hey", "howdy");
        double avg = (Double) words.stream()
            .collect(Collectors.averagingDouble(s -> (double) ((String) s).length()));
        // (2+5+3+5)/4 = 15/4 = 3.75 → round down → 3
        return (int) avg;  // 3
    }
}
