import java.util.concurrent.ConcurrentHashMap;

public final class ConcurrentHashMapComputeTest {
    public static int computeIfAbsentMissAndHit() {
        ConcurrentHashMap<String, String> map = new ConcurrentHashMap<>();
        int[] calls = {0};
        String first = map.computeIfAbsent("k", key -> {
            calls[0]++;
            return key + "!";
        });
        String second = map.computeIfAbsent("k", key -> {
            calls[0]++;
            return key + "?";
        });
        if (!"k!".equals(first)) return -1;
        if (!"k!".equals(second)) return -2;
        return calls[0];
    }

    public static int computeIfPresentHit() {
        ConcurrentHashMap<String, String> map = new ConcurrentHashMap<>();
        map.put("k", "v");
        String value = map.computeIfPresent("k", (key, old) -> old + "!");
        return "v!".equals(value) && "v!".equals(map.get("k")) ? 1 : -1;
    }

    public static int computeIfPresentMiss() {
        ConcurrentHashMap<String, String> map = new ConcurrentHashMap<>();
        int[] calls = {0};
        String value = map.computeIfPresent("missing", (key, old) -> {
            calls[0]++;
            return "bad";
        });
        return value == null && calls[0] == 0 && map.isEmpty() ? 1 : -1;
    }

    public static int computeUpdatesValue() {
        ConcurrentHashMap<String, String> map = new ConcurrentHashMap<>();
        map.put("k", "k!");
        String value = map.compute("k", (key, old) -> old + "?");
        return "k!?".equals(value) && "k!?".equals(map.get("k")) ? 1 : -1;
    }

    public static int mergeAccumulatesCount() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.merge("count", 1, Integer::sum);
        map.merge("count", 1, Integer::sum);
        map.merge("count", 1, Integer::sum);
        return map.get("count");
    }

    public static int forEachVisitsSnapshot() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("only", 42);
        int[] seen = {0};
        map.forEach((key, value) -> {
            if ("only".equals(key) && value == 42) {
                seen[0]++;
            }
        });
        return seen[0];
    }
}
