import java.util.ArrayList;
import java.util.Comparator;
import java.util.Collections;

public class ComparatorTest {
    static int testNaturalOrderSort() {
        ArrayList<String> list = new ArrayList<>();
        list.add("banana");
        list.add("apple");
        list.add("cherry");
        list.sort(Comparator.naturalOrder());
        return ((String) list.get(0)).length();  // "apple".length() = 5
    }

    static int testReverseOrderSort() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(3);
        list.add(1);
        list.add(2);
        list.sort(Comparator.reverseOrder());
        return (Integer) list.get(0);  // 3
    }

    static int testCollectionsSortWithComparator() {
        ArrayList<String> list = new ArrayList<>();
        list.add("cc");
        list.add("a");
        list.add("bbb");
        Collections.sort(list, Comparator.comparingInt(String::length));
        return ((String) list.get(0)).length();  // "a".length() = 1
    }
}
