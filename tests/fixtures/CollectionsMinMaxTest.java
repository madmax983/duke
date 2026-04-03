import java.util.ArrayList;
import java.util.Collections;

public class CollectionsMinMaxTest {
    static int testMinInt() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(3);
        list.add(1);
        list.add(4);
        list.add(1);
        list.add(5);
        return (Integer) Collections.min(list);  // 1
    }

    static int testMaxInt() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(3);
        list.add(1);
        list.add(4);
        return (Integer) Collections.max(list);  // 4
    }

    static int testMinString() {
        ArrayList<String> list = new ArrayList<>();
        list.add("banana");
        list.add("apple");
        list.add("cherry");
        return ((String) Collections.min(list)).length();  // "apple".length() = 5
    }

    static int testShuffle() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(1);
        list.add(2);
        list.add(3);
        Collections.shuffle(list);
        return list.size();  // 3 (size unchanged)
    }
}
