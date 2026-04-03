import java.util.HashSet;
import java.util.HashMap;

public class HashSetIterTest {
    static int testHashSetForEach() {
        HashSet<String> set = new HashSet<>();
        set.add("a");
        set.add("b");
        set.add("c");
        int count = 0;
        for (String s : set) {
            count++;
        }
        return count;  // 3
    }

    static int testKeySetForEach() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", 1);
        map.put("y", 2);
        int count = 0;
        for (String k : map.keySet()) {
            count++;
        }
        return count;  // 2
    }

    static int testSumValues() {
        HashSet<Integer> nums = new HashSet<>();
        nums.add(10);
        nums.add(20);
        nums.add(30);
        int sum = 0;
        for (int n : nums) {
            sum += n;
        }
        return sum;  // 60
    }
}
