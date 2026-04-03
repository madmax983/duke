import java.util.*;
import java.util.stream.*;

public class Phase45Test {

    // ---- List.forEach ----

    static int testListForEach() {
        List<Integer> list = Arrays.asList(
            Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3));
        int[] sum = {0};
        list.forEach(x -> sum[0] += ((Integer) x).intValue());
        return sum[0];  // 6
    }

    // ---- Stream.sorted(Comparator) ----

    static int testStreamSortedComparator() {
        List<String> list = new ArrayList<>(Arrays.asList("banana", "apple", "cherry"));
        List<String> sorted = list.stream()
            .sorted(Comparator.comparing(s -> (String) s))
            .collect(Collectors.toList());
        return sorted.get(0).equals("apple") ? 1 : 0;  // 1
    }

    static int testStreamSortedComparatorReversed() {
        List<Integer> list = Arrays.asList(
            Integer.valueOf(3), Integer.valueOf(1), Integer.valueOf(4), Integer.valueOf(1));
        List<Integer> sorted = list.stream()
            .sorted(Comparator.comparingInt((Integer x) -> -x.intValue()))
            .collect(Collectors.toList());
        return ((Integer) sorted.get(0)).intValue();  // 4 (largest first)
    }

    // ---- Arrays.toString ----

    static int testArraysToStringInt() {
        int[] arr = {1, 2, 3};
        return Arrays.toString(arr).equals("[1, 2, 3]") ? 1 : 0;  // 1
    }

    static int testArraysToStringObject() {
        String[] arr = {"a", "b", "c"};
        return Arrays.toString(arr).equals("[a, b, c]") ? 1 : 0;  // 1
    }

    // ---- Map.replace ----

    static int testMapReplace() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", Integer.valueOf(1));
        map.replace("x", Integer.valueOf(42));
        return ((Integer) map.get("x")).intValue();  // 42
    }

    static int testMapReplaceMissing() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", Integer.valueOf(1));
        Object result = map.replace("y", Integer.valueOf(99)); // key not present
        return result == null ? 1 : 0;  // 1
    }

    // ---- Collections.swap ----

    static int testCollectionsSwap() {
        List<String> list = new ArrayList<>(Arrays.asList("a", "b", "c"));
        Collections.swap(list, 0, 2);
        return list.get(0).equals("c") && list.get(2).equals("a") ? 1 : 0;  // 1
    }

    // ---- Collectors.partitioningBy ----

    @SuppressWarnings("unchecked")
    static int testCollectorsPartitioningBy() {
        List<Integer> list = Arrays.asList(
            Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3),
            Integer.valueOf(4), Integer.valueOf(5));
        Map result = (Map) list.stream().collect(
            Collectors.partitioningBy(x -> ((Integer) x).intValue() % 2 == 0));
        List evens = (List) result.get(Boolean.TRUE);
        List odds  = (List) result.get(Boolean.FALSE);
        return evens.size() * 10 + odds.size();  // 2*10 + 3 = 23
    }

    // ---- IntStream.sorted ----

    static int testIntStreamSorted() {
        int[] result = IntStream.of(5, 3, 1, 4, 2).sorted().toArray();
        return result[0] + result[4];  // 1 + 5 = 6
    }

    // ---- Map.getOrDefault with missing key ----

    static int testMapGetOrDefault() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(1));
        return ((Integer) map.getOrDefault("b", Integer.valueOf(99))).intValue();  // 99
    }

    // ---- String.format %n ----

    static int testStringFormatNewline() {
        String s = String.format("a%nb");
        return s.contains("\n") ? 1 : 0;  // 1
    }

    // ---- Collections.unmodifiableMap ----

    static int testCollectionsUnmodifiableMap() {
        HashMap<String, Integer> m = new HashMap<>();
        m.put("k", Integer.valueOf(7));
        Map<String, Integer> um = Collections.unmodifiableMap(m);
        return ((Integer) um.get("k")).intValue();  // 7
    }
}
