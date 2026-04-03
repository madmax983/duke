import java.util.*;
import java.util.stream.*;

public class Phase44Test {

    // ---- Arrays.copyOfRange ----

    static int testArraysCopyOfRangeInt() {
        int[] arr = {1, 2, 3, 4, 5};
        int[] sub = Arrays.copyOfRange(arr, 1, 4);  // [2,3,4]
        return sub[0] + sub[1] + sub[2];  // 9
    }

    static int testArraysCopyOfRangeObject() {
        String[] arr = {"a", "b", "c", "d"};
        String[] sub = Arrays.copyOfRange(arr, 1, 3);  // ["b","c"]
        return sub[0].equals("b") && sub[1].equals("c") ? 1 : 0;  // 1
    }

    static int testArraysCopyOfRangePad() {
        int[] arr = {1, 2, 3};
        int[] padded = Arrays.copyOfRange(arr, 1, 6);  // [2,3,0,0,0]
        return padded[0] + padded[1] + padded[2] + padded[3] + padded[4];  // 5
    }

    // ---- List.subList ----

    static int testListSubList() {
        List<Integer> list = new ArrayList<>(Arrays.asList(
            Integer.valueOf(10), Integer.valueOf(20),
            Integer.valueOf(30), Integer.valueOf(40), Integer.valueOf(50)));
        List<Integer> sub = list.subList(1, 4);
        return sub.size();  // 3
    }

    static int testListSubListGet() {
        List<String> list = new ArrayList<>(Arrays.asList("a", "b", "c", "d"));
        List<String> sub = list.subList(1, 3);
        return sub.get(0).equals("b") ? 1 : 0;  // 1
    }

    // ---- Comparator.reversed ----

    static int testComparatorReversed() {
        List<String> list = new ArrayList<>(Arrays.asList("banana", "apple", "cherry"));
        list.sort(Comparator.comparing((String s) -> s).reversed());
        return list.get(0).equals("cherry") ? 1 : 0;  // 1 (reverse order)
    }

    // ---- Map.entrySet stream ----

    static int testMapEntrySetStream() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("a", Integer.valueOf(1));
        map.put("b", Integer.valueOf(2));
        map.put("c", Integer.valueOf(3));
        // sum of all values via entrySet stream
        int[] sum = {0};
        map.entrySet().stream()
            .forEach(e -> sum[0] += ((Map.Entry) e).getValue() instanceof Integer
                ? ((Integer) ((Map.Entry) e).getValue()).intValue() : 0);
        return sum[0];  // 6
    }

    // ---- String.intern ----

    static int testStringIntern() {
        String a = new String("hello");
        return a.intern().equals("hello") ? 1 : 0;  // 1 (intern returns a string equal to the original)
    }

    // ---- Collections.binarySearch ----

    static int testCollectionsBinarySearch() {
        List<Integer> list = new ArrayList<>(Arrays.asList(
            Integer.valueOf(1), Integer.valueOf(3), Integer.valueOf(5),
            Integer.valueOf(7), Integer.valueOf(9)));
        return Collections.binarySearch(list, Integer.valueOf(5));  // 2
    }

    static int testCollectionsBinarySearchMiss() {
        List<Integer> list = new ArrayList<>(Arrays.asList(
            Integer.valueOf(1), Integer.valueOf(3), Integer.valueOf(5)));
        int idx = Collections.binarySearch(list, Integer.valueOf(4));
        return idx < 0 ? 1 : 0;  // 1 (not found returns negative)
    }

    // ---- Iterator.remove (via removeIf) ----

    static int testRemoveIf() {
        List<Integer> list = new ArrayList<>(Arrays.asList(
            Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3),
            Integer.valueOf(4), Integer.valueOf(5)));
        list.removeIf(x -> ((Integer) x).intValue() % 2 == 0);
        return list.size();  // 3 (odd numbers remain)
    }

    // ---- String.chars() advanced ----

    static int testStringDistinctChars() {
        long count = "aabbcc".chars().distinct().count();
        return (int) count;  // 3
    }

    // ---- Map.values().stream() ----

    static int testMapValuesStream() {
        HashMap<String, Integer> map = new HashMap<>();
        map.put("x", Integer.valueOf(10));
        map.put("y", Integer.valueOf(20));
        map.put("z", Integer.valueOf(30));
        int total = map.values().stream()
            .mapToInt(v -> ((Integer) v).intValue())
            .sum();
        return total;  // 60
    }
}
