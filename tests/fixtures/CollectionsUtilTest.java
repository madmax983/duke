import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public class CollectionsUtilTest {
    static int testEmptyList() {
        List list = Collections.emptyList();
        return list.size();  // 0
    }

    static int testSingletonList() {
        List list = Collections.singletonList(42);
        return list.size();  // 1
    }

    static int testSingletonListGet() {
        List list = Collections.singletonList(99);
        return (Integer) list.get(0);  // 99
    }

    static int testReverse() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(1);
        list.add(2);
        list.add(3);
        Collections.reverse(list);
        return (Integer) list.get(0);  // 3
    }

    static int testReverseSize() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(10);
        list.add(20);
        Collections.reverse(list);
        return list.size();  // 2
    }

    static int testFrequency() {
        ArrayList<Integer> list = new ArrayList<>();
        list.add(1);
        list.add(2);
        list.add(1);
        list.add(3);
        return Collections.frequency(list, 1);  // 2
    }
}
