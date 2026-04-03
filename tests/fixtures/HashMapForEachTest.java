import java.util.HashMap;

public class HashMapForEachTest {
    static int testForEachCount() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", 1);
        map.put("b", 2);
        map.put("c", 3);
        int[] count = {0};
        map.forEach((k, v) -> count[0]++);
        return count[0];  // 3
    }

    static int testForEachSumValues() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", 10);
        map.put("y", 20);
        int[] sum = {0};
        map.forEach((k, v) -> sum[0] += (Integer) v);
        return sum[0];  // 30
    }
}
