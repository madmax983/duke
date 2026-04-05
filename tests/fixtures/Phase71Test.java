import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase71Test {

    // Map.getOrDefault
    public static int testMapGetOrDefault() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        return m.getOrDefault("a", 0) + m.getOrDefault("b", 99); // 1 + 99 = 100
    }

    // Map.putIfAbsent
    public static int testMapPutIfAbsent() {
        Map<String, Integer> m = new HashMap<>();
        m.put("x", 10);
        m.putIfAbsent("x", 99);  // should not change
        m.putIfAbsent("y", 20);  // should insert
        return m.get("x") + m.get("y"); // 10 + 20 = 30
    }

    // Map.merge
    public static int testMapMerge() {
        Map<String, Integer> m = new HashMap<>();
        m.put("k", 5);
        m.merge("k", 3, Integer::sum); // 5 + 3 = 8
        m.merge("new", 7, Integer::sum); // insert 7
        return m.get("k") + m.get("new"); // 8 + 7 = 15
    }

    // Map.compute
    public static int testMapCompute() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 10);
        m.compute("a", (k, v) -> v == null ? 1 : v + 5); // 10 + 5 = 15
        m.compute("b", (k, v) -> v == null ? 1 : v + 5); // null -> 1
        return m.get("a") + m.get("b"); // 15 + 1 = 16
    }

    // LinkedHashMap preserves insertion order
    public static int testLinkedHashMap() {
        LinkedHashMap<String, Integer> m = new LinkedHashMap<>();
        m.put("first", 1);
        m.put("second", 2);
        m.put("third", 3);
        int sum = 0;
        for (int v : m.values()) sum += v;
        return sum; // 6
    }

    // List.indexOf
    public static int testListIndexOf() {
        List<String> list = new ArrayList<>();
        list.add("a"); list.add("b"); list.add("c"); list.add("b");
        return list.indexOf("b"); // 1
    }

    // List.subList
    public static int testListSubList() {
        List<Integer> list = new ArrayList<>();
        for (int i = 0; i < 5; i++) list.add(i * 2); // [0,2,4,6,8]
        List<Integer> sub = list.subList(1, 4); // [2,4,6]
        int sum = 0;
        for (int v : sub) sum += v;
        return sum; // 12
    }

    // Collections.reverse
    public static int testCollectionsReverse() {
        List<Integer> list = new ArrayList<>();
        list.add(1); list.add(2); list.add(3); list.add(4);
        Collections.reverse(list);
        return list.get(0) * 10 + list.get(3); // 4*10 + 1 = 41
    }
}
