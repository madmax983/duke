import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase56Test {

    // ---- Collectors.minBy ----

    static int testCollectorsMinBy() {
        Optional<String> result = Stream.of("banana", "apple", "cherry")
            .collect(Collectors.minBy(Comparator.comparingInt(s -> ((String) s).length())));
        return result.get().length();  // "apple" length=5
    }

    static int testCollectorsMinByNatural() {
        Optional<Integer> result = Stream.of(
                Integer.valueOf(3), Integer.valueOf(1), Integer.valueOf(4), Integer.valueOf(1), Integer.valueOf(5))
            .collect(Collectors.minBy(Comparator.comparingInt(n -> ((Integer) n).intValue())));
        return result.get().intValue();  // 1
    }

    // ---- Collectors.maxBy ----

    static int testCollectorsMaxBy() {
        Optional<String> result = Stream.of("hi", "hello", "hey")
            .collect(Collectors.maxBy(Comparator.comparingInt(s -> ((String) s).length())));
        return result.get().length();  // "hello" length=5
    }

    static int testCollectorsMaxByInt() {
        Optional<Integer> result = Stream.of(
                Integer.valueOf(3), Integer.valueOf(1), Integer.valueOf(4), Integer.valueOf(1), Integer.valueOf(5))
            .collect(Collectors.maxBy(Comparator.comparingInt(n -> ((Integer) n).intValue())));
        return result.get().intValue();  // 5
    }

    // ---- Collectors.summingDouble ----

    static int testCollectorsSummingDouble() {
        List<String> words = Arrays.asList("hi", "hello", "hey");
        double sum = (Double) words.stream()
            .collect(Collectors.summingDouble(s -> (double) ((String) s).length()));
        return (int) sum;  // 2+5+3 = 10
    }

    // ---- Collectors.averagingLong ----

    static int testCollectorsAveragingLong() {
        List<String> words = Arrays.asList("hi", "hello", "hey", "howdy");
        double avg = (Double) words.stream()
            .collect(Collectors.averagingLong(s -> (long) ((String) s).length()));
        // (2+5+3+5)/4 = 15/4 = 3.75 → floor → 3
        return (int) avg;  // 3
    }

    // ---- Collectors.toUnmodifiableMap ----

    static int testCollectorsToUnmodifiableMap() {
        Map<String, Integer> result = Stream.of("hi", "hello", "hey")
            .collect(Collectors.toUnmodifiableMap(
                s -> (String) s,
                s -> Integer.valueOf(((String) s).length())
            ));
        return ((Integer) result.get("hello")).intValue();  // 5
    }

    static int testCollectorsToUnmodifiableMapSize() {
        Map<Integer, String> result = Stream.of("a", "bb", "ccc")
            .collect(Collectors.toUnmodifiableMap(
                s -> Integer.valueOf(((String) s).length()),
                s -> ((String) s).toUpperCase()
            ));
        return result.size();  // 3
    }

    // ---- Collectors.collectingAndThen ----

    static int testCollectorsCollectingAndThen() {
        // collect to list, then get size
        Integer size = (Integer) Stream.of("a", "b", "c", "d")
            .collect(Collectors.collectingAndThen(
                Collectors.toList(),
                list -> Integer.valueOf(((List) list).size())
            ));
        return size.intValue();  // 4
    }

    static int testCollectorsCollectingAndThenJoin() {
        // collect to joining, then check length
        String result = (String) Stream.of("x", "y", "z")
            .collect(Collectors.collectingAndThen(
                Collectors.joining("-"),
                s -> ((String) s).toUpperCase()
            ));
        return result.equals("X-Y-Z") ? 1 : 0;  // 1
    }

    static int testCollectorsCollectingAndThenCount() {
        // collect to unmodifiable list, count elements > 2 chars
        List<String> words = Stream.of("hi", "hello", "hey", "howdy")
            .collect(Collectors.collectingAndThen(
                Collectors.toList(),
                list -> (List) list
            ));
        int count = 0;
        for (int i = 0; i < words.size(); i++) {
            if (words.get(i).length() > 2) count++;
        }
        return count;  // "hello"=5, "hey"=3, "howdy"=5 → 3
    }
}
