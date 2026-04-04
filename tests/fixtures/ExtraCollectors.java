import java.util.stream.*;
import java.util.*;

public class ExtraCollectors {
    public static int runTests() {
        // GroupingBy2Collector
        Map<Integer, Long> m = Stream.of("a", "bb", "c", "dd", "eee")
            .collect(Collectors.groupingBy(String::length, Collectors.counting()));
        if (m.get(1) != 2L || m.get(2) != 2L || m.get(3) != 1L) return 1;

        // MinByCollector
        Optional<String> min = Stream.of("bb", "a", "ccc").collect(Collectors.minBy((a, b) -> a.length() - b.length()));
        if (!min.isPresent() || !min.get().equals("a")) return 2;

        // MaxByCollector
        Optional<String> max = Stream.of("bb", "a", "ccc").collect(Collectors.maxBy((a, b) -> a.length() - b.length()));
        if (!max.isPresent() || !max.get().equals("ccc")) return 3;

        // SummingDoubleCollector
        double dSum = Stream.of("a", "bb").collect(Collectors.summingDouble(String::length));
        if (dSum != 3.0) return 4;

        // AveragingLongCollector
        double lAvg = Stream.of("a", "bb", "ccc").collect(Collectors.averagingLong(String::length));
        if (lAvg != 2.0) return 5;

        // ReducingNoIdentityCollector
        Optional<String> redOpt = Stream.of("a", "b", "c").collect(Collectors.reducing((a, b) -> a + b));
        if (!redOpt.isPresent() || !redOpt.get().equals("abc")) return 6;

        // ReducingCollector
        String red = Stream.of("a", "b", "c").collect(Collectors.reducing("", (a, b) -> a + b));
        if (!red.equals("abc")) return 7;

        // ReducingMappingCollector
        String redMap = Stream.of("a", "b", "c").collect(Collectors.reducing("", s -> s, (a, b) -> a + b));
        if (!redMap.equals("abc")) return 8;

        // CollectingAndThenCollector
        List<String> unmodifiable = Stream.of("a", "b", "c").collect(Collectors.collectingAndThen(Collectors.toList(), Collections::unmodifiableList));
        if (unmodifiable.size() != 3) return 9;

        // ToMapMergeCollector
        Map<Integer, String> mapMerge = Stream.of("a", "a", "bb").collect(Collectors.toMap(String::length, s -> s, (v1, v2) -> v1 + v2));
        if (!mapMerge.get(1).equals("aa") || !mapMerge.get(2).equals("bb")) return 10;

        // PartitioningByCollector
        Map<Boolean, List<String>> part = Stream.of("a", "bb", "c", "dd").collect(Collectors.partitioningBy(s -> s.length() == 2));
        if (part.get(true).size() != 2 || part.get(false).size() != 2) return 11;

        // MappingCollector
        List<Integer> mapCollect = Stream.of("a", "bb").collect(Collectors.mapping(String::length, Collectors.toList()));
        if (mapCollect.get(0) != 1 || mapCollect.get(1) != 2) return 12;

        // SummingIntCollector
        int iSum = Stream.of("a", "bb").collect(Collectors.summingInt(String::length));
        if (iSum != 3) return 13;

        // AveragingIntCollector
        double iAvg = Stream.of("a", "bb", "ccc").collect(Collectors.averagingInt(String::length));
        if (iAvg != 2.0) return 14;

        // SummingLongCollector
        long lSum = Stream.of("a", "bb").collect(Collectors.summingLong(String::length));
        if (lSum != 3L) return 15;

        // AveragingDoubleCollector
        double dAvg = Stream.of("a", "bb", "ccc").collect(Collectors.averagingDouble(String::length));
        if (dAvg != 2.0) return 16;

        return 0;
    }
}
