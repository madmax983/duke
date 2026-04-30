import java.util.concurrent.ConcurrentHashMap;

public final class ConcurrentHashMapStaticInitTest {
    private static final ConcurrentHashMap<String, Integer> CACHE = new ConcurrentHashMap<>();

    static {
        CACHE.put("answer", 42);
        CACHE.computeIfAbsent("doubled", key -> CACHE.get("answer") * 2);
    }

    public static int readCache() {
        return CACHE.get("answer") + CACHE.get("doubled");
    }

    public static void printCache() {
        System.out.println(CACHE.get("answer") + " " + CACHE.get("doubled"));
    }

    public static void main(String[] args) {
        printCache();
    }
}
