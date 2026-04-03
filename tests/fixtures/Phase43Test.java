import java.util.*;
import java.util.stream.*;

public class Phase43Test {

    // ---- Stream.mapToInt ----

    static int testStreamMapToInt() {
        List<String> words = Arrays.asList("hi", "hey", "hello");
        return words.stream()
            .mapToInt(s -> ((String) s).length())
            .sum();  // 2+3+5 = 10
    }

    static int testStreamMapToIntMax() {
        List<String> words = Arrays.asList("hi", "hey", "hello");
        return words.stream()
            .mapToInt(s -> ((String) s).length())
            .max().getAsInt();  // 5
    }

    // ---- Collectors.toSet ----

    @SuppressWarnings("unchecked")
    static int testCollectorsToSet() {
        Set result = (Set) Stream.of("a", "b", "a", "c")
            .collect(Collectors.toSet());
        return result.size();  // 3 (deduped)
    }

    // ---- Collectors.toMap ----

    @SuppressWarnings("unchecked")
    static int testCollectorsToMap() {
        Map result = (Map) Stream.of("hi", "hey", "hello")
            .collect(Collectors.toMap(
                s -> (Object) s,
                s -> (Object) Integer.valueOf(((String) s).length())
            ));
        return result.size();  // 3
    }

    @SuppressWarnings("unchecked")
    static int testCollectorsToMapGet() {
        Map result = (Map) Stream.of("hi", "hey")
            .collect(Collectors.toMap(
                s -> (Object) s,
                s -> (Object) Integer.valueOf(((String) s).length())
            ));
        return ((Integer) result.get("hi")).intValue();  // 2
    }

    // ---- Stream.min / max with Comparator ----

    static int testStreamMinComparator() {
        Optional result = Stream.of("banana", "apple", "cherry")
            .min(Comparator.comparing(s -> (String) s));
        return result.isPresent() ? ((String) result.get()).length() : 0;  // "apple".length()=5
    }

    static int testStreamMaxComparator() {
        Optional result = Stream.of("banana", "apple", "cherry")
            .max(Comparator.comparing(s -> (String) s));
        return result.isPresent() ? ((String) result.get()).length() : 0;  // "cherry".length()=6
    }

    // ---- IntStream.reduce ----

    static int testIntStreamReduce() {
        return IntStream.range(1, 6)
            .reduce(0, (a, b) -> a + b);  // 1+2+3+4+5 = 15
    }

    static int testIntStreamReduceOptional() {
        return IntStream.of(3, 1, 4, 1, 5)
            .reduce((a, b) -> a + b)
            .getAsInt();  // 14
    }

    // ---- Stream.reduce ----

    static int testStreamReduce() {
        Optional result = Stream.of("a", "b", "c")
            .reduce((a, b) -> (String) a + (String) b);
        return result.isPresent() ? ((String) result.get()).length() : 0;  // "abc".length()=3
    }

    static int testStreamReduceIdentity() {
        String result = (String) Stream.of("x", "y", "z")
            .reduce("", (a, b) -> (String) a + (String) b);
        return result.length();  // "xyz".length()=3
    }

    // ---- Arrays.sort(Object[]) ----

    static int testArraysSortObjects() {
        String[] arr = {"banana", "apple", "cherry"};
        Arrays.sort(arr);
        return arr[0].equals("apple") ? 1 : 0;  // 1
    }

    // ---- String.valueOf(char[]) ----

    static int testStringValueOfCharArray() {
        char[] chars = {'h', 'e', 'l', 'l', 'o'};
        return String.valueOf(chars).equals("hello") ? 1 : 0;  // 1
    }

    // ---- new String(char[]) ----

    static int testNewStringFromCharArray() {
        char[] chars = {'h', 'i'};
        return new String(chars).equals("hi") ? 1 : 0;  // 1
    }

    // ---- Collections.frequency ----

    static int testCollectionsFrequencyAlreadyDone() {
        List<String> list = Arrays.asList("a", "b", "a", "c", "a");
        return Collections.frequency(list, "a");  // 3
    }
}
