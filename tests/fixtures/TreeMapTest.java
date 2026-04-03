import java.util.TreeMap;

public class TreeMapTest {
    static int testSize() {
        TreeMap<String, Integer> map = new TreeMap<>();
        map.put("b", 2);
        map.put("a", 1);
        map.put("c", 3);
        return map.size();  // 3
    }

    static int testGet() {
        TreeMap<String, Integer> map = new TreeMap<>();
        map.put("key", 42);
        return (Integer) map.get("key");  // 42
    }

    static int testFirstKey() {
        TreeMap<String, Integer> map = new TreeMap<>();
        map.put("b", 2);
        map.put("a", 1);
        map.put("c", 3);
        return map.firstKey().length();  // "a".length() = 1
    }

    static int testLastKey() {
        TreeMap<String, Integer> map = new TreeMap<>();
        map.put("b", 2);
        map.put("a", 1);
        map.put("c", 3);
        return map.lastKey().length();  // "c".length() = 1
    }

    static int testContainsKey() {
        TreeMap<String, Integer> map = new TreeMap<>();
        map.put("x", 10);
        return map.containsKey("x") ? 1 : 0;  // 1
    }
}
