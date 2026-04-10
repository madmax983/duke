import java.util.*;
import java.util.stream.*;

public class Phase90Test {

    // ---- String.toCharArray + char arithmetic ----
    public static int testCharArraySum() {
        char[] chars = "ABC".toCharArray();
        int sum = 0;
        for (char c : chars) sum += (c - 'A'); // 0+1+2 = 3
        return sum;
    }

    // ---- Arrays.asList + stream ----
    public static int testArraysAsListStream() {
        List<Integer> list = Arrays.asList(1, 2, 3, 4, 5);
        return list.stream()
            .mapToInt(Integer::intValue)
            .sum(); // 15
    }

    // ---- Collections.reverse ----
    public static int testCollectionsReverse() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        Collections.reverse(list);
        return list.get(0) + list.get(4); // 5 + 1 = 6
    }

    // ---- Collections.min / max ----
    public static int testCollectionsMinMax() {
        List<Integer> list = Arrays.asList(3, 1, 4, 1, 5, 9, 2, 6);
        return Collections.min(list) + Collections.max(list); // 1 + 9 = 10
    }

    // ---- String.substring chaining ----
    public static int testSubstringChain() {
        String s = "HelloWorld";
        String a = s.substring(0, 5);   // "Hello"
        String b = s.substring(5);       // "World"
        return a.length() + b.length();  // 5 + 5 = 10
    }

    // ---- Static fields on custom class ----
    static class Config {
        static final int MAX = 100;
        static final int MIN = 1;
        static int current = 50;
    }

    public static int testStaticFields() {
        Config.current = Config.MIN + Config.MAX; // 101
        return Config.current; // 101
    }

    // ---- List.set ----
    public static int testListSet() {
        List<Integer> list = new ArrayList<>(Arrays.asList(10, 20, 30));
        list.set(1, 99);
        return list.get(0) + list.get(1) + list.get(2); // 10 + 99 + 30 = 139
    }

    // ---- Iterator over array-backed list ----
    public static int testIteratorSum() {
        List<Integer> list = new ArrayList<>(Arrays.asList(5, 10, 15, 20));
        int sum = 0;
        Iterator<Integer> it = list.iterator();
        while (it.hasNext()) {
            sum += it.next();
        }
        return sum; // 50
    }

    // ---- Ternary in stream ----
    public static int testTernaryInStream() {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5, 6);
        int sum = nums.stream()
            .mapToInt(n -> n % 2 == 0 ? n * 2 : n)
            .sum(); // 1 + 4 + 3 + 8 + 5 + 12 = 33
        return sum;
    }

    // ---- String.intern / equality ----
    public static int testStringEquality() {
        String a = "hello";
        String b = "hel" + "lo";
        String c = new StringBuilder("hel").append("lo").toString();
        int result = 0;
        if (a.equals(b)) result += 1;
        if (a.equals(c)) result += 10;
        if (b.equals(c)) result += 100;
        return result; // 111
    }
}
