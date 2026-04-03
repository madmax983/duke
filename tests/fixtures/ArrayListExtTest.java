import java.util.ArrayList;

public class ArrayListExtTest {
    static int testRemoveAt() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(10);
        list.add(20);
        list.add(30);
        list.remove(1);  // removes 20
        return (Integer) list.get(1);  // now 30
    }

    static int testRemoveAtFirst() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(5);
        list.add(9);
        list.remove(0);  // removes 5
        return list.size();  // 1
    }

    static int testRemoveObj() {
        ArrayList<String> list = new ArrayList<>();
        list.add("a");
        list.add("b");
        list.add("c");
        boolean removed = list.remove("b");
        return removed ? list.size() : -1;  // 2
    }

    static int testContainsTrue() {
        ArrayList<String> list = new ArrayList<>();
        list.add("hello");
        list.add("world");
        return list.contains("world") ? 1 : 0;  // 1
    }

    static int testContainsFalse() {
        ArrayList<String> list = new ArrayList<>();
        list.add("foo");
        return list.contains("bar") ? 1 : 0;  // 0
    }

    static int testClear() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(1);
        list.add(2);
        list.add(3);
        list.clear();
        return list.size();  // 0
    }

    static int testIsEmpty() {
        ArrayList<Integer> list = new ArrayList<>();
        int before = list.isEmpty() ? 1 : 0;
        list.add(42);
        int after = list.isEmpty() ? 1 : 0;
        return before + after;  // 1 + 0 = 1
    }

    static int testSet() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(10);
        list.add(20);
        list.add(30);
        list.set(1, 99);
        return (Integer) list.get(1);  // 99
    }

    static int testIndexOf() {
        ArrayList<String> list = new ArrayList<>();
        list.add("x");
        list.add("y");
        list.add("z");
        return list.indexOf("y");  // 1
    }

    static int testIndexOfMissing() {
        ArrayList<String> list = new ArrayList<>();
        list.add("a");
        return list.indexOf("b");  // -1
    }

    static int testAddAt() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(1);
        list.add(3);
        list.add(1, 2);  // insert 2 at index 1
        return (Integer) list.get(1);  // 2
    }
}
