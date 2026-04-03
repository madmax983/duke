import java.util.*;
import java.util.stream.*;

public class Phase51Test {

    // ---- LongStream.of / sum ----

    static int testLongStreamOf() {
        long sum = LongStream.of(1L, 2L, 3L, 4L, 5L).sum();
        return (int) sum;  // 15
    }

    // ---- LongStream.range ----

    static int testLongStreamRange() {
        long count = LongStream.range(0L, 5L).count();
        return (int) count;  // 5
    }

    // ---- LongStream.filter ----

    static int testLongStreamFilter() {
        long sum = LongStream.of(1L, 2L, 3L, 4L, 5L)
            .filter(n -> n % 2 == 0)
            .sum();
        return (int) sum;  // 2+4 = 6
    }

    // ---- LongStream.map ----

    static int testLongStreamMap() {
        long sum = LongStream.of(1L, 2L, 3L)
            .map(n -> n * n)
            .sum();
        return (int) sum;  // 1+4+9 = 14
    }

    // ---- LongStream.min / max ----

    static int testLongStreamMin() {
        return (int) LongStream.of(3L, 1L, 4L, 1L, 5L).min().getAsLong();  // 1
    }

    static int testLongStreamMax() {
        return (int) LongStream.of(3L, 1L, 4L, 1L, 5L).max().getAsLong();  // 5
    }

    // ---- LongStream.count ----

    static int testLongStreamCount() {
        return (int) LongStream.of(10L, 20L, 30L).count();  // 3
    }

    // ---- LongStream.average ----

    static int testLongStreamAverage() {
        double avg = LongStream.of(1L, 2L, 3L, 4L, 5L).average().getAsDouble();
        return (int) avg;  // 3
    }

    // ---- LongStream.toArray ----

    static int testLongStreamToArray() {
        long[] arr = LongStream.of(5L, 3L, 1L).toArray();
        return (int)(arr[0] + arr[1] + arr[2]);  // 9
    }

    // ---- LongStream.sorted ----

    static int testLongStreamSorted() {
        long[] arr = LongStream.of(5L, 3L, 1L, 4L, 2L).sorted().toArray();
        return (int)(arr[0] + arr[4]);  // 1 + 5 = 6
    }

    // ---- LongStream.boxed → Stream<Long> ----

    static int testLongStreamBoxed() {
        long count = LongStream.of(1L, 2L, 3L).boxed().count();
        return (int) count;  // 3
    }

    // ---- LongStream.reduce ----

    static int testLongStreamReduce() {
        long result = LongStream.of(1L, 2L, 3L, 4L, 5L)
            .reduce(0L, (a, b) -> a + b);
        return (int) result;  // 15
    }

    // ---- DoubleStream.of / sum ----

    static int testDoubleStreamOf() {
        double sum = DoubleStream.of(1.0, 2.0, 3.0).sum();
        return (int) sum;  // 6
    }

    // ---- DoubleStream.filter ----

    static int testDoubleStreamFilter() {
        long count = DoubleStream.of(1.5, 2.5, 3.5, 4.5)
            .filter(d -> d > 2.0)
            .count();
        return (int) count;  // 3
    }

    // ---- DoubleStream.map ----

    static int testDoubleStreamMap() {
        double sum = DoubleStream.of(1.0, 4.0, 9.0)
            .map(d -> Math.sqrt(d))
            .sum();
        return (int) Math.round(sum);  // 1+2+3 = 6
    }

    // ---- DoubleStream.min / max ----

    static int testDoubleStreamMin() {
        return (int) DoubleStream.of(3.0, 1.0, 4.0).min().getAsDouble();  // 1
    }

    static int testDoubleStreamMax() {
        return (int) DoubleStream.of(3.0, 1.0, 4.0).max().getAsDouble();  // 4
    }

    // ---- DoubleStream.count ----

    static int testDoubleStreamCount() {
        return (int) DoubleStream.of(1.0, 2.0, 3.0).count();  // 3
    }

    // ---- DoubleStream.average ----

    static int testDoubleStreamAverage() {
        double avg = DoubleStream.of(2.0, 4.0, 6.0).average().getAsDouble();
        return (int) avg;  // 4
    }

    // ---- DoubleStream.toArray ----

    static int testDoubleStreamToArray() {
        double[] arr = DoubleStream.of(1.0, 2.0, 3.0).toArray();
        return (int)(arr[0] + arr[1] + arr[2]);  // 6
    }

    // ---- IntStream.asLongStream ----

    static int testIntStreamAsLongStream() {
        long sum = IntStream.of(1, 2, 3, 4, 5).asLongStream().sum();
        return (int) sum;  // 15
    }

    // ---- IntStream.asDoubleStream ----

    static int testIntStreamAsDoubleStream() {
        double sum = IntStream.of(1, 4, 9).asDoubleStream()
            .map(d -> Math.sqrt(d))
            .sum();
        return (int) Math.round(sum);  // 1+2+3 = 6
    }

    // ---- Collectors.summingInt ----

    static int testCollectorsSummingInt() {
        List<String> words = Arrays.asList("hi", "hey", "hello");
        int total = words.stream()
            .collect(Collectors.summingInt(s -> ((String) s).length()));
        return total;  // 2+3+5 = 10
    }

    // ---- Collectors.averagingInt ----

    static int testCollectorsAveragingInt() {
        List<String> words = Arrays.asList("hi", "hey", "hello");
        double avg = words.stream()
            .collect(Collectors.averagingInt(s -> ((String) s).length()));
        return (int) avg;  // (2+3+5)/3 = 3 (truncated)
    }
}
