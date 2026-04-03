import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase50Test {

    // ---- Stream.distinct ----

    static int testStreamDistinct() {
        long count = Stream.of("a", "b", "a", "c", "b").distinct().count();
        return (int) count;  // 3
    }

    static int testStreamDistinctCollect() {
        List result = Stream.of("x", "y", "x", "z").distinct()
            .collect(Collectors.toList());
        return result.size();  // 3
    }

    // ---- Stream.findFirst ----

    static int testStreamFindFirst() {
        Optional result = Stream.of("apple", "banana", "cherry")
            .filter(s -> ((String) s).length() > 5)
            .findFirst();
        return result.isPresent() ? ((String) result.get()).length() : 0;  // "banana".length() = 6
    }

    static int testStreamFindFirstEmpty() {
        Optional result = Stream.<String>of().findFirst();
        return result.isPresent() ? 1 : 0;  // 0
    }

    // ---- Stream.allMatch ----

    static int testStreamAllMatch() {
        boolean result = Stream.of("apple", "avocado", "almond")
            .allMatch(s -> ((String) s).startsWith("a"));
        return result ? 1 : 0;  // 1
    }

    static int testStreamAllMatchFail() {
        boolean result = Stream.of("apple", "banana", "almond")
            .allMatch(s -> ((String) s).startsWith("a"));
        return result ? 1 : 0;  // 0
    }

    // ---- Stream.anyMatch ----

    static int testStreamAnyMatch() {
        boolean result = Stream.of("apple", "banana", "cherry")
            .anyMatch(s -> ((String) s).startsWith("b"));
        return result ? 1 : 0;  // 1
    }

    static int testStreamAnyMatchFail() {
        boolean result = Stream.of("apple", "avocado")
            .anyMatch(s -> ((String) s).startsWith("z"));
        return result ? 1 : 0;  // 0
    }

    // ---- Stream.noneMatch ----

    static int testStreamNoneMatch() {
        boolean result = Stream.of("apple", "avocado", "almond")
            .noneMatch(s -> ((String) s).startsWith("z"));
        return result ? 1 : 0;  // 1
    }

    static int testStreamNoneMatchFail() {
        boolean result = Stream.of("apple", "banana")
            .noneMatch(s -> ((String) s).startsWith("b"));
        return result ? 1 : 0;  // 0
    }

    // ---- Collections.emptyList / emptySet / emptyMap ----

    static int testCollectionsEmptyList() {
        List<String> list = Collections.emptyList();
        return list.size();  // 0
    }

    static int testCollectionsEmptySet() {
        Set<String> set = Collections.emptySet();
        return set.size();  // 0
    }

    static int testCollectionsEmptyMap() {
        Map<String, Integer> map = Collections.emptyMap();
        return map.size();  // 0
    }

    // ---- Collections.singletonList ----

    static int testCollectionsSingletonList() {
        List<String> list = Collections.singletonList("hello");
        return list.size() == 1 && list.get(0).equals("hello") ? 1 : 0;  // 1
    }

    // ---- Objects.isNull / nonNull ----

    static int testObjectsIsNull() {
        return Objects.isNull(null) ? 1 : 0;  // 1
    }

    static int testObjectsNonNull() {
        return Objects.nonNull("hello") ? 1 : 0;  // 1
    }

    // ---- Objects.requireNonNull ----

    static int testObjectsRequireNonNull() {
        String s = (String) Objects.requireNonNull("hello");
        return s.length();  // 5
    }
}
