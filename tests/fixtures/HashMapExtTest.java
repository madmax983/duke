import java.util.HashMap;

public class HashMapExtTest {
    static int testPutIfAbsentNew() {
        HashMap<String, Integer> map = new HashMap<>();
        Object prev = map.putIfAbsent("key", 42);
        return prev == null ? (Integer) map.get("key") : -1;  // 42
    }

    static int testPutIfAbsentExisting() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("key", 100);
        Integer prev = (Integer) map.putIfAbsent("key", 999);
        return prev;  // 100 (old value returned; not overwritten)
    }

    static int testPutIfAbsentNoOverwrite() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", 7);
        map.putIfAbsent("x", 42);
        return (Integer) map.get("x");  // 7 (unchanged)
    }

    static int testClear() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        map.clear();
        return map.size();  // 0
    }

    static int testContainsValueTrue() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", 42);
        map.put("y", 99);
        return map.containsValue(42) ? 1 : 0;  // 1
    }

    static int testContainsValueFalse() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", 1);
        map.put("y", 2);
        return map.containsValue(99) ? 1 : 0;  // 0
    }
}
