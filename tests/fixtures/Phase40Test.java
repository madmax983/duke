import java.util.stream.IntStream;
import java.util.stream.Stream;
import java.util.List;

public class Phase40Test {

    // ---- IntStream.range / rangeClosed ----

    static int testIntStreamRangeSum() {
        return IntStream.range(1, 6).sum();  // 1+2+3+4+5 = 15
    }

    static int testIntStreamRangeClosedSum() {
        return IntStream.rangeClosed(1, 5).sum();  // 1+2+3+4+5 = 15
    }

    static int testIntStreamRangeCount() {
        return (int) IntStream.range(0, 10).count();  // 10
    }

    static int testIntStreamRangeClosedCount() {
        return (int) IntStream.rangeClosed(1, 5).count();  // 5
    }

    // ---- IntStream operations ----

    static int testIntStreamFilter() {
        return (int) IntStream.range(1, 11).filter(n -> n % 2 == 0).count();  // 5 evens
    }

    static int testIntStreamMap() {
        return IntStream.range(1, 4).map(n -> n * n).sum();  // 1+4+9 = 14
    }

    static int testIntStreamForEachCount() {
        int[] acc = {0};
        IntStream.range(0, 5).forEach(n -> acc[0] += n);
        return acc[0];  // 0+1+2+3+4 = 10
    }

    static int testIntStreamMin() {
        return IntStream.of(5, 3, 8, 1, 4).min().getAsInt();  // 1
    }

    static int testIntStreamMax() {
        return IntStream.of(5, 3, 8, 1, 4).max().getAsInt();  // 8
    }

    static int testIntStreamAverage() {
        double avg = IntStream.rangeClosed(1, 4).average().getAsDouble();
        return (int) avg;  // (1+2+3+4)/4 = 2 (int truncation)
    }

    static int testIntStreamToArray() {
        int[] arr = IntStream.range(1, 4).toArray();
        return arr[0] + arr[1] + arr[2];  // 1+2+3 = 6
    }

    static int testIntStreamBoxed() {
        List<Integer> list = IntStream.range(1, 4).boxed()
            .collect(java.util.stream.Collectors.toList());
        return list.size();  // 3
    }

    // ---- IntStream.of ----

    static int testIntStreamOf() {
        return IntStream.of(10, 20, 30).sum();  // 60
    }

    // ---- Stream.limit / skip ----

    static int testStreamLimit() {
        return (int) Stream.of("a", "b", "c", "d", "e").limit(3).count();  // 3
    }

    static int testStreamSkip() {
        return (int) Stream.of("a", "b", "c", "d", "e").skip(2).count();  // 3
    }

    static int testStreamSkipLimit() {
        return (int) Stream.of("a", "b", "c", "d", "e").skip(1).limit(3).count();  // 3
    }

    // ---- Stream.flatMap ----

    static int testStreamFlatMap() {
        List<List> nested = java.util.Arrays.asList(
            java.util.Arrays.asList("a", "b"),
            java.util.Arrays.asList("c", "d")
        );
        return (int) nested.stream()
            .flatMap(inner -> ((java.util.List) inner).stream())
            .count();  // 4
    }

    // ---- IntStream.mapToObj ----

    static int testIntStreamMapToObj() {
        long count = IntStream.range(1, 4)
            .mapToObj(n -> Integer.toString(n))
            .count();
        return (int) count;  // 3
    }
}
