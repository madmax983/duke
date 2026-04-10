import java.util.*;

public class Phase66Test {

    // HashMap.replaceAll — doubles each value
    public static int testHashMapReplaceAll() {
        HashMap<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        m.put("b", 2);
        m.put("c", 3);
        m.replaceAll((k, v) -> v * 10);
        int sum = 0;
        for (Object v : m.values()) sum += (Integer) v;
        return sum; // 10+20+30 = 60
    }

    // HashMap.replaceAll — appends key name to value
    public static int testHashMapReplaceAllLength() {
        HashMap<String, String> m = new HashMap<>();
        m.put("hello", "world");
        m.replaceAll((k, v) -> k + v); // "helloworld"
        Object val = m.get("hello");
        if (val instanceof String) {
            return ((String) val).length(); // expect 10
        }
        return -1;
    }

    // Iterator.remove — removes even numbers
    public static int testIteratorRemove() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(1);
        list.add(2);
        list.add(3);
        list.add(4);
        list.add(5);
        Iterator<Integer> it = list.iterator();
        while (it.hasNext()) {
            Integer v = it.next();
            if (v % 2 == 0) it.remove();
        }
        return list.size(); // expect 3 (1,3,5 remain)
    }

    // Iterator.remove — removes all elements
    public static int testIteratorRemoveAll() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(10);
        list.add(20);
        list.add(30);
        Iterator<Integer> it = list.iterator();
        while (it.hasNext()) {
            it.next();
            it.remove();
        }
        return list.size(); // expect 0
    }

    // Iterator.remove then continue iteration
    public static int testIteratorRemoveSum() {
        ArrayList<Integer> list = new ArrayList<>();
        for (int i = 1; i <= 6; i++) list.add(i);
        // Remove odds, sum evens
        Iterator<Integer> it = list.iterator();
        while (it.hasNext()) {
            Integer v = it.next();
            if (v % 2 != 0) it.remove();
        }
        int sum = 0;
        for (Integer v : list) sum += v;
        return sum; // 2+4+6 = 12
    }
}
