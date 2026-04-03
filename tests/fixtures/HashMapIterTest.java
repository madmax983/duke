import java.util.HashMap;
import java.util.Map;
import java.util.Set;

public class HashMapIterTest {
    static int testKeySet() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        map.put("c", 3);
        Set<String> keys = map.keySet();
        return keys.size();  // 3
    }

    static int testKeySetContains() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("hello", 42);
        map.put("world", 99);
        Set<String> keys = map.keySet();
        int count = 0;
        for (String k : keys) {
            if (k.equals("hello") || k.equals("world")) count++;
        }
        return count;  // 2
    }

    static int testValues() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", 10);
        map.put("y", 20);
        map.put("z", 30);
        int sum = 0;
        for (Object v : map.values()) {
            sum += (Integer) v;
        }
        return sum;  // 60
    }

    static int testEntrySet() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("one", 1);
        map.put("two", 2);
        int sum = 0;
        for (Map.Entry<String, Integer> e : map.entrySet()) {
            sum += (Integer) e.getValue();
        }
        return sum;  // 3
    }

    static int testEntrySetKeys() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("alpha", 100);
        int len = 0;
        for (Map.Entry<String, Integer> e : map.entrySet()) {
            len = ((String) e.getKey()).length();
        }
        return len;  // 5 ("alpha".length())
    }
}
