import java.util.HashSet;

public class HashSetTest {
    static int testAddAndContains() {
        HashSet<String> set = new HashSet<>();
        set.add("hello");
        set.add("world");
        if (set.contains("hello") && set.contains("world")) return 1;
        return 0;
    }

    static int testSize() {
        HashSet<Integer> set = new HashSet<>();
        set.add(Integer.valueOf(1));
        set.add(Integer.valueOf(2));
        set.add(Integer.valueOf(3));
        return set.size();  // 3
    }

    static int testNoDuplicates() {
        HashSet<String> set = new HashSet<>();
        set.add("x");
        set.add("x");  // duplicate — ignored
        set.add("y");
        return set.size();  // 2
    }

    static int testRemove() {
        HashSet<String> set = new HashSet<>();
        set.add("a");
        set.add("b");
        boolean removed = set.remove("a");
        if (removed && set.size() == 1 && !set.contains("a")) return 1;
        return 0;
    }

    static int testIsEmpty() {
        HashSet<String> set = new HashSet<>();
        if (!set.isEmpty()) return 0;
        set.add("item");
        if (set.isEmpty()) return 0;
        return 1;
    }

    static int testAddReturnsFalse() {
        HashSet<String> set = new HashSet<>();
        boolean first = set.add("dup");
        boolean second = set.add("dup");
        if (first && !second) return 1;
        return 0;
    }
}
