import java.util.*;

public class Phase79Test {

    // List.of() — immutable list factory
    public static int testListOf() {
        List<Integer> list = List.of(1, 2, 3, 4, 5);
        return list.size(); // 5
    }

    public static int testListOfGet() {
        List<String> list = List.of("a", "b", "c");
        return list.get(1).length(); // "b".length() = 1
    }

    public static int testListOfEmpty() {
        List<Integer> list = List.of();
        return list.size(); // 0
    }

    public static int testListOfContains() {
        List<Integer> list = List.of(10, 20, 30);
        return list.contains(20) ? 1 : 0; // 1
    }

    // Set.of()
    public static int testSetOf() {
        Set<String> set = Set.of("x", "y", "z");
        return set.size(); // 3
    }

    public static int testSetOfContains() {
        Set<Integer> set = Set.of(1, 2, 3);
        return set.contains(2) ? 1 : 0; // 1
    }

    // Map.of()
    public static int testMapOf() {
        Map<String, Integer> map = Map.of("a", 1, "b", 2, "c", 3);
        return map.size(); // 3
    }

    public static int testMapOfGet() {
        Map<String, Integer> map = Map.of("key", 42);
        Object v = map.get("key");
        if (v instanceof Integer) return (Integer) v; // 42
        return -1;
    }

    // Objects.requireNonNull
    public static int testObjectsRequireNonNull() {
        try {
            Objects.requireNonNull(null, "must not be null");
            return 0;
        } catch (NullPointerException e) {
            return 1; // expect 1
        }
    }

    public static int testObjectsRequireNonNullPass() {
        String s = Objects.requireNonNull("hello", "msg");
        return s.length(); // 5
    }

    // Objects.equals
    public static int testObjectsEquals() {
        int r = 0;
        if (Objects.equals("a", "a")) r += 1;
        if (Objects.equals(null, null)) r += 2;
        if (!Objects.equals("a", null)) r += 4;
        if (!Objects.equals(null, "b")) r += 8;
        return r; // 15
    }

    // Objects.isNull / nonNull
    public static int testObjectsIsNull() {
        int r = 0;
        if (Objects.isNull(null)) r += 1;
        if (!Objects.isNull("x")) r += 2;
        if (Objects.nonNull("y")) r += 4;
        if (!Objects.nonNull(null)) r += 8;
        return r; // 15
    }

    // Objects.toString
    public static int testObjectsToString() {
        String s = Objects.toString(null, "default");
        return s.length(); // "default".length() = 7
    }

    // Collections.emptyList / emptySet / emptyMap
    public static int testCollectionsEmpty() {
        List<Integer> l = Collections.emptyList();
        Set<String> s = Collections.emptySet();
        Map<String, Integer> m = Collections.emptyMap();
        return l.size() + s.size() + m.size(); // 0
    }

    // Collections.singletonList (already partial, but test here)
    public static int testCollectionsSingletonList() {
        List<String> l = Collections.singletonList("only");
        return l.size() + l.get(0).length(); // 1 + 4 = 5
    }
}
