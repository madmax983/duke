import java.util.HashMap;

public class HashMapTest {
    static int testPutAndGet() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(10));
        map.put("b", Integer.valueOf(20));
        Integer va = (Integer) map.get("a");
        Integer vb = (Integer) map.get("b");
        return va.intValue() + vb.intValue();  // 30
    }

    static int testSize() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", Integer.valueOf(1));
        map.put("y", Integer.valueOf(2));
        map.put("z", Integer.valueOf(3));
        return map.size();  // 3
    }

    static int testContainsKey() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("key", Integer.valueOf(42));
        if (map.containsKey("key") && !map.containsKey("missing")) return 1;
        return 0;
    }

    static int testGetMissing() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(1));
        if (map.get("nope") == null) return 1;
        return 0;
    }

    static int testRemove() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(5));
        map.put("b", Integer.valueOf(6));
        map.remove("a");
        return map.size();  // 1
    }

    static int testIsEmpty() {
        HashMap<String, Integer> map = new HashMap<>();
        if (!map.isEmpty()) return 0;
        map.put("k", Integer.valueOf(1));
        if (map.isEmpty()) return 0;
        return 1;
    }

    static int testOverwrite() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("k", Integer.valueOf(1));
        map.put("k", Integer.valueOf(99));
        return map.size();  // 1 (overwrite, not new entry)
    }

    static int testGetOrDefault() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(7));
        Integer v1 = (Integer) map.getOrDefault("a", Integer.valueOf(0));
        Integer v2 = (Integer) map.getOrDefault("missing", Integer.valueOf(99));
        return v1.intValue() + v2.intValue();  // 106
    }
}
