import java.util.concurrent.ConcurrentHashMap;

public final class ConcurrentHashMapNullTest {
    public static int putNullKeyThrows() {
        ConcurrentHashMap<String, String> map = new ConcurrentHashMap<>();
        try {
            map.put(null, "x");
            return -1;
        } catch (NullPointerException expected) {
            return 1;
        }
    }

    public static int putNullValueThrows() {
        ConcurrentHashMap<String, String> map = new ConcurrentHashMap<>();
        try {
            map.put("k", null);
            return -1;
        } catch (NullPointerException expected) {
            return 1;
        }
    }

    public static int getNullThrows() {
        ConcurrentHashMap<String, String> map = new ConcurrentHashMap<>();
        try {
            map.get(null);
            return -1;
        } catch (NullPointerException expected) {
            return 1;
        }
    }

    public static int containsKeyNullThrows() {
        ConcurrentHashMap<String, String> map = new ConcurrentHashMap<>();
        try {
            map.containsKey(null);
            return -1;
        } catch (NullPointerException expected) {
            return 1;
        }
    }

    public static int putIfAbsentNullValueThrows() {
        ConcurrentHashMap<String, String> map = new ConcurrentHashMap<>();
        try {
            map.putIfAbsent("k", null);
            return -1;
        } catch (NullPointerException expected) {
            return 1;
        }
    }

    public static int mergeNullValueThrows() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        try {
            map.merge("count", null, Integer::sum);
            return -1;
        } catch (NullPointerException expected) {
            return 1;
        }
    }
}
