import java.util.TreeSet;

public class TreeSetTest {
    static int testSize() {
        TreeSet<String> set = new TreeSet<>();
        set.add("b");
        set.add("a");
        set.add("c");
        return set.size();  // 3
    }

    static int testFirst() {
        TreeSet<String> set = new TreeSet<>();
        set.add("b");
        set.add("a");
        set.add("c");
        return set.first().length();  // "a".length() = 1
    }

    static int testLast() {
        TreeSet<String> set = new TreeSet<>();
        set.add("b");
        set.add("a");
        set.add("c");
        return set.last().length();  // "c".length() = 1
    }

    static int testContains() {
        TreeSet<Integer> set = new TreeSet<>();
        set.add(1);
        set.add(2);
        set.add(3);
        return set.contains(2) ? 1 : 0;  // 1
    }

    static int testNoDuplicates() {
        TreeSet<String> set = new TreeSet<>();
        set.add("a");
        set.add("a");
        set.add("b");
        return set.size();  // 2
    }
}
