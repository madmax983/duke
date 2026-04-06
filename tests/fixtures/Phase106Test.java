import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase106Test {

    // ---- Nested generic collections ----
    public static int testNestedCollections() {
        Map<String, List<Integer>> m = new HashMap<>();
        m.put("evens", Arrays.asList(2, 4, 6));
        m.put("odds", Arrays.asList(1, 3, 5));
        int sum = 0;
        for (List<Integer> list : m.values()) {
            for (int v : list) sum += v;
        }
        return sum; // 2+4+6+1+3+5 = 21
    }

    // ---- Stream.flatMap ----
    public static int testStreamFlatMap() {
        List<List<Integer>> nested = Arrays.asList(
            Arrays.asList(1, 2, 3),
            Arrays.asList(4, 5, 6)
        );
        return nested.stream()
            .flatMap(Collection::stream)
            .mapToInt(Integer::intValue)
            .sum(); // 21
    }

    // ---- Iterator over LinkedList ----
    public static int testLinkedListIterator() {
        LinkedList<Integer> list = new LinkedList<>(Arrays.asList(10, 20, 30, 40, 50));
        int sum = 0;
        Iterator<Integer> it = list.iterator();
        while (it.hasNext()) {
            sum += it.next();
        }
        return sum; // 150
    }

    // ---- Collections.unmodifiableMap ----
    public static int testUnmodifiableMap() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1); m.put("b", 2);
        Map<String, Integer> um = Collections.unmodifiableMap(m);
        int result = um.get("a") + um.get("b"); // 3
        try {
            um.put("c", 3);
        } catch (UnsupportedOperationException e) {
            result += 10; // 13
        }
        return result;
    }

    // ---- String.matches regex ----
    public static int testStringMatches() {
        String s = "hello123";
        boolean allAlnum = s.matches("[a-z0-9]+");
        boolean allAlpha = s.matches("[a-z]+");
        return (allAlnum ? 1 : 0) + (allAlpha ? 0 : 1); // 1 + 1 = 2
    }

    // ---- Stream.anyMatch / allMatch ----
    public static int testStreamMatchOps() {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        boolean any = nums.stream().anyMatch(n -> n > 4);  // true (5)
        boolean all = nums.stream().allMatch(n -> n > 0);  // true
        boolean none = nums.stream().noneMatch(n -> n > 10); // true
        return (any ? 1 : 0) + (all ? 1 : 0) + (none ? 1 : 0); // 3
    }

    // ---- Integer.sum / Integer.max / Integer.min ----
    public static int testIntegerStaticMethods() {
        int sum = Integer.sum(3, 7);     // 10
        int max = Integer.max(3, 7);     // 7
        int min = Integer.min(3, 7);     // 3
        return sum + max + min; // 20
    }

    // ---- String.replaceAll ----
    public static int testStringReplaceAll() {
        String s = "hello world foo bar";
        String result = s.replaceAll("\s+", "-");
        return result.length(); // "hello-world-foo-bar" = 19
    }

    // ---- Collectors.joining(delimiter, prefix, suffix) ----
    public static int testCollectorsJoiningFull() {
        List<String> words = Arrays.asList("a", "b", "c");
        String result = words.stream()
            .collect(Collectors.joining(", ", "[", "]"));
        return result.length(); // "[a, b, c]" = 9
    }

    // ---- Map.keySet() ----
    public static int testMapKeySet() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1); m.put("b", 2); m.put("c", 3);
        return m.keySet().size(); // 3
    }
}
