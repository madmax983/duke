import java.util.LinkedHashMap;

public class LinkedHashMapTest {
    static int testSize() {
        LinkedHashMap<String, Integer> map = new LinkedHashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        map.put("c", 3);
        return map.size();  // 3
    }

    static int testGet() {
        LinkedHashMap<String, Integer> map = new LinkedHashMap<>();
        map.put("key", 42);
        return (Integer) map.get("key");  // 42
    }

    static int testContainsKey() {
        LinkedHashMap<String, Integer> map = new LinkedHashMap<>();
        map.put("x", 10);
        return map.containsKey("x") ? 1 : 0;  // 1
    }

    static int testInsertionOrder() {
        LinkedHashMap<String, Integer> map = new LinkedHashMap<>();
        map.put("first", 1);
        map.put("second", 2);
        map.put("third", 3);
        // Iterate and get first key
        String firstKey = (String) map.keySet().iterator().next();
        return firstKey.length();  // "first".length() = 5
    }
}
