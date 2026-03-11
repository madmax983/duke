import java.util.*;

public class CollectionsSortTest {
    public static void main(String[] args) {
        // Integer sort (exercises callback → Integer.compareTo native)
        List<Integer> ints = new ArrayList<>();
        ints.add(3); ints.add(1); ints.add(4); ints.add(1); ints.add(5);
        Collections.sort(ints);
        for (int x : ints) System.out.println(x);

        // String sort (exercises callback → String.compareTo native)
        List<String> strs = new ArrayList<>();
        strs.add("banana");
        strs.add("apple");
        strs.add("cherry");
        Collections.sort(strs);
        for (String s : strs) System.out.println(s);
    }
}
