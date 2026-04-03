import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase48Test {

    // ---- Stream.generate ----

    static int testStreamGenerate() {
        // generate 5 copies of 1 and sum them
        int sum = Stream.generate(() -> Integer.valueOf(1))
            .limit(5)
            .mapToInt(x -> ((Integer) x).intValue())
            .sum();
        return sum;  // 5
    }

    // ---- Stream.iterate ----

    static int testStreamIterate() {
        // iterate 0, 1, 2, 3, 4 and sum
        int sum = Stream.iterate(Integer.valueOf(0), x -> Integer.valueOf(((Integer) x).intValue() + 1))
            .limit(5)
            .mapToInt(x -> ((Integer) x).intValue())
            .sum();
        return sum;  // 0+1+2+3+4 = 10
    }

    // ---- Stream.concat ----

    static int testStreamConcat() {
        Stream<Integer> a = Stream.of(Integer.valueOf(1), Integer.valueOf(2));
        Stream<Integer> b = Stream.of(Integer.valueOf(3), Integer.valueOf(4), Integer.valueOf(5));
        long count = Stream.concat(a, b).count();
        return (int) count;  // 5
    }

    // ---- Stream.empty ----

    static int testStreamEmpty() {
        long count = Stream.empty().count();
        return (int) count;  // 0
    }

    // ---- IntStream.rangeClosed ----

    static int testIntStreamRangeClosed() {
        return IntStream.rangeClosed(1, 5).sum();  // 1+2+3+4+5 = 15
    }

    // ---- Map.compute ----

    static int testMapCompute() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("k", Integer.valueOf(10));
        map.compute("k", (key, val) -> Integer.valueOf(
            ((Integer) val).intValue() * 2));
        return ((Integer) map.get("k")).intValue();  // 20
    }

    static int testMapComputeAbsent() {
        HashMap<String, Integer> map = new HashMap<>();
        map.compute("k", (key, val) -> Integer.valueOf(99));
        return ((Integer) map.get("k")).intValue();  // 99
    }

    // ---- Optional.map ----

    static int testOptionalMap() {
        Optional<String> opt = Optional.of("hello");
        Optional<Integer> mapped = opt.map(s -> Integer.valueOf(((String) s).length()));
        return mapped.isPresent() ? ((Integer) mapped.get()).intValue() : 0;  // 5
    }

    // ---- Optional.filter ----

    static int testOptionalFilter() {
        Optional<Integer> opt = Optional.of(Integer.valueOf(42));
        Optional<Integer> filtered = opt.filter(x -> ((Integer) x).intValue() > 10);
        return filtered.isPresent() ? 1 : 0;  // 1
    }

    static int testOptionalFilterEmpty() {
        Optional<Integer> opt = Optional.of(Integer.valueOf(5));
        Optional<Integer> filtered = opt.filter(x -> ((Integer) x).intValue() > 10);
        return filtered.isPresent() ? 1 : 0;  // 0
    }

    // ---- Optional.orElse ----

    static int testOptionalOrElse() {
        Optional<Integer> opt = Optional.empty();
        Integer result = (Integer) opt.orElse(Integer.valueOf(99));
        return result.intValue();  // 99
    }

    // ---- Optional.orElseGet ----

    static int testOptionalOrElseGet() {
        Optional<Integer> opt = Optional.empty();
        Integer result = (Integer) opt.orElseGet(() -> Integer.valueOf(77));
        return result.intValue();  // 77
    }
}
