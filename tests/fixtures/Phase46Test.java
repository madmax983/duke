import java.util.*;
import java.util.stream.*;

public class Phase46Test {

    // ---- String.repeat (Java 11) ----

    static int testStringRepeat() {
        return "ab".repeat(3).equals("ababab") ? 1 : 0;  // 1
    }

    // ---- String.isBlank (Java 11) ----

    static int testStringIsBlank() {
        return "   ".isBlank() ? 1 : 0;  // 1
    }

    static int testStringIsBlankNot() {
        return "hi".isBlank() ? 0 : 1;  // 1
    }

    // ---- String.lines() (Java 11) ----

    static int testStringLines() {
        long count = "a\nb\nc".lines().count();
        return (int) count;  // 3
    }

    // ---- Stream.takeWhile (Java 9) ----

    static int testStreamTakeWhile() {
        List<Integer> result = Stream.of(
            Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3),
            Integer.valueOf(4), Integer.valueOf(5))
            .takeWhile(x -> ((Integer) x).intValue() < 4)
            .collect(Collectors.toList());
        return result.size();  // 3
    }

    // ---- Stream.dropWhile (Java 9) ----

    static int testStreamDropWhile() {
        List<Integer> result = Stream.of(
            Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3),
            Integer.valueOf(4), Integer.valueOf(5))
            .dropWhile(x -> ((Integer) x).intValue() < 4)
            .collect(Collectors.toList());
        return result.size();  // 2
    }

    // ---- Map.putIfAbsent ----

    static int testMapPutIfAbsent() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", Integer.valueOf(1));
        map.putIfAbsent("x", Integer.valueOf(99));  // should not replace
        map.putIfAbsent("y", Integer.valueOf(2));    // should insert
        return ((Integer) map.get("x")).intValue() + ((Integer) map.get("y")).intValue();  // 3
    }

    // ---- Map.computeIfAbsent ----

    static int testMapComputeIfAbsent() {
        HashMap<String, Integer> map = new HashMap<>();
        map.computeIfAbsent("key", k -> Integer.valueOf(42));
        return ((Integer) map.get("key")).intValue();  // 42
    }

    // ---- Map.merge ----

    static int testMapMerge() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("k", Integer.valueOf(10));
        map.merge("k", Integer.valueOf(5), (a, b) -> Integer.valueOf(
            ((Integer) a).intValue() + ((Integer) b).intValue()));
        return ((Integer) map.get("k")).intValue();  // 15
    }

    // ---- Collections.reverseOrder ----

    static int testCollectionsReverseOrder() {
        List<Integer> list = new ArrayList<>(Arrays.asList(
            Integer.valueOf(3), Integer.valueOf(1), Integer.valueOf(2)));
        list.sort(Collections.reverseOrder());
        return ((Integer) list.get(0)).intValue();  // 3 (largest first)
    }

    // ---- Optional.ofNullable ----

    static int testOptionalOfNullable() {
        Optional<String> opt = Optional.ofNullable("hello");
        return opt.isPresent() ? 1 : 0;  // 1
    }

    static int testOptionalOfNullableNull() {
        Optional<String> opt = Optional.ofNullable(null);
        return opt.isPresent() ? 1 : 0;  // 0
    }
}
