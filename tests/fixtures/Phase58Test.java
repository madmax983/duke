import java.util.*;
import java.util.stream.*;

public class Phase58Test {

    // ---- Comparator.comparingDouble ----

    static int testComparatorComparingDouble() {
        List<String> words = Arrays.asList("hello", "hi", "hey", "howdy");
        words.sort(Comparator.comparingDouble(s -> (double) ((String) s).length()));
        return ((String) words.get(0)).length();  // "hi"=2, smallest
    }

    static int testComparatorComparingDoubleReversed() {
        List<String> words = Arrays.asList("hello", "hi", "hey");
        words.sort(Comparator.comparingDouble((String s) -> (double) s.length()).reversed());
        return ((String) words.get(0)).length();  // "hello"=5, largest first
    }

    // ---- Map.copyOf ----

    static int testMapCopyOf() {
        Map<String, Integer> original = new HashMap<>();
        original.put("a", Integer.valueOf(1));
        original.put("b", Integer.valueOf(2));
        original.put("c", Integer.valueOf(3));
        Map<String, Integer> copy = Map.copyOf(original);
        return copy.size();  // 3
    }

    static int testMapCopyOfContents() {
        Map<String, Integer> original = new HashMap<>();
        original.put("hello", Integer.valueOf(5));
        Map<String, Integer> copy = Map.copyOf(original);
        return ((Integer) copy.get("hello")).intValue();  // 5
    }

    // ---- Map.entry ----

    static int testMapEntry() {
        Map.Entry<String, Integer> e = Map.entry("key", Integer.valueOf(42));
        int k = e.getKey().length();
        int v = ((Integer) e.getValue()).intValue();
        return k + v;  // 3 + 42 = 45
    }

    // ---- Map.ofEntries ----

    static int testMapOfEntries() {
        Map<String, Integer> m = Map.ofEntries(
            Map.entry("a", Integer.valueOf(1)),
            Map.entry("b", Integer.valueOf(2)),
            Map.entry("c", Integer.valueOf(3))
        );
        return m.size();  // 3
    }

    static int testMapOfEntriesGet() {
        Map<String, Integer> m = Map.ofEntries(
            Map.entry("x", Integer.valueOf(10)),
            Map.entry("y", Integer.valueOf(20))
        );
        return ((Integer) m.get("y")).intValue();  // 20
    }

    // ---- Collections.singletonMap ----

    static int testCollectionsSingletonMap() {
        Map<String, Integer> m = Collections.singletonMap("only", Integer.valueOf(99));
        return m.size() + ((Integer) m.get("only")).intValue();  // 1 + 99 = 100
    }

    // ---- Collections.singletonSet ----

    static int testCollectionsSingletonSet() {
        Set<String> s = Collections.singleton("hello");
        int sz = s.size();
        int has = s.contains("hello") ? 1 : 0;
        return sz + has;  // 1 + 1 = 2
    }

    // ---- Collections.unmodifiableSet ----

    static int testCollectionsUnmodifiableSet() {
        Set<String> original = new HashSet<>();
        original.add("a");
        original.add("b");
        original.add("c");
        Set<String> unmod = Collections.unmodifiableSet(original);
        return unmod.size();  // 3
    }

    static int testCollectionsUnmodifiableSetContains() {
        Set<Integer> original = new HashSet<>();
        original.add(Integer.valueOf(42));
        Set<Integer> unmod = Collections.unmodifiableSet(original);
        return unmod.contains(Integer.valueOf(42)) ? 1 : 0;  // 1
    }
}
