import java.util.HashMap;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentMap;
import java.util.concurrent.ConcurrentHashMap;

public final class ConcurrentHashMapBasicTest {
    public static int defaultCtorPutGetSize() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        map.put("c", 3);
        map.put("d", 4);
        map.put("e", 5);
        if (map.size() != 5) return -1;
        return map.get("a") + map.get("b") + map.get("c") + map.get("d") + map.get("e");
    }

    public static int capacityCtor() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>(32);
        map.put("x", 7);
        return map.get("x");
    }

    public static int capacityLoadCtor() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>(32, 0.75f);
        map.put("x", 8);
        return map.get("x");
    }

    public static int capacityLoadConcurrencyCtor() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>(32, 0.75f, 4);
        map.put("x", 9);
        return map.get("x");
    }

    public static int copyCtorCopiesExistingMap() {
        Map<String, Integer> source = new HashMap<>();
        source.put("a", 10);
        source.put("b", 20);
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>(source);
        return map.get("a") + map.get("b") + map.size();
    }

    public static int putAllCopiesEntries() {
        Map<String, Integer> source = new HashMap<>();
        source.put("a", 1);
        source.put("b", 2);
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.putAll(source);
        return map.get("a") + map.get("b") + map.size();
    }

    public static int mapInterfaceDispatchesToConcurrentHashMap() {
        Map<String, Integer> map = new ConcurrentHashMap<>();
        map.put("m", 41);
        map.put("m", 42);
        return map.get("m");
    }

    public static int mapInterfaceKeySetDispatchesToConcurrentHashMap() {
        Map<String, Integer> map = new ConcurrentHashMap<>();
        map.put("key", 1);
        return map.keySet().contains("key") ? 1 : -1;
    }

    public static int concurrentMapInterfaceDispatchesToConcurrentHashMap() {
        ConcurrentMap<String, Integer> map = new ConcurrentHashMap<>();
        Integer missing = map.putIfAbsent("cm", 24);
        Integer existing = map.putIfAbsent("cm", 99);
        return (missing == null ? 10 : 100) + existing + map.get("cm");
    }

    public static int containsKeyTrueFalse() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("present", 1);
        return (map.containsKey("present") ? 1 : 0) + (map.containsKey("missing") ? 10 : 0);
    }

    public static int containsValueTrueFalse() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 11);
        map.put("b", 22);
        return (map.containsValue(22) ? 1 : 0) + (map.containsValue(99) ? 10 : 0);
    }

    public static int getOrDefaultHitMiss() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 5);
        return map.getOrDefault("a", 99) + map.getOrDefault("b", 40);
    }

    public static int removeReturnsPriorValue() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 12);
        Integer old = map.remove("a");
        return old + (map.containsKey("a") ? 100 : 0) + map.size();
    }

    public static int conditionalRemoveBranches() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        boolean miss = map.remove("a", 99);
        boolean hit = map.remove("a", 1);
        return (miss ? 100 : 0) + (hit ? 10 : 0) + map.size();
    }

    public static int putIfAbsentBranches() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 1);
        Integer existing = map.putIfAbsent("a", 99);
        Integer absent = map.putIfAbsent("b", 2);
        return existing + (absent == null ? 10 : 100) + map.get("a") + map.get("b");
    }

    public static int replaceValueBranches() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 4);
        Integer old = map.replace("a", 5);
        Integer missing = map.replace("b", 6);
        return old + map.get("a") + (missing == null ? 10 : 100);
    }

    public static int replaceCasBranches() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 7);
        boolean miss = map.replace("a", 99, 100);
        boolean hit = map.replace("a", 7, 8);
        return (miss ? 100 : 0) + (hit ? 10 : 0) + map.get("a");
    }

    public static int clearThenIsEmpty() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        map.clear();
        return (map.isEmpty() ? 1 : 0) + map.size();
    }

    public static int snapshotViews() {
        ConcurrentHashMap<String, Integer> map = new ConcurrentHashMap<>();
        map.put("a", 1);
        Set<String> keys = map.keySet();
        int valuesSize = map.values().size();
        int entriesSize = map.entrySet().size();
        map.put("b", 2);
        if (keys.contains("b")) return -1;
        return (keys.contains("a") ? 10 : 0) + valuesSize + entriesSize;
    }
}
