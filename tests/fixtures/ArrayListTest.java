import java.util.ArrayList;

public class ArrayListTest {
    static int testSize() {
        ArrayList<String> list = new ArrayList<>();
        list.add("a");
        list.add("bb");
        list.add("ccc");
        return list.size();  // expect 3
    }

    static int testGet() {
        ArrayList<String> list = new ArrayList<>();
        list.add("hello");
        list.add("world");
        String s = list.get(1);
        return s.length();  // "world" = 5
    }

    static int testForEachCount() {
        ArrayList<String> list = new ArrayList<>();
        list.add("a");
        list.add("b");
        list.add("c");
        int count = 0;
        for (String s : list) {
            count++;
        }
        return count;  // expect 3
    }

    static int testForEachSum() {
        ArrayList<String> list = new ArrayList<>();
        list.add("hi");
        list.add("there");
        list.add("x");
        int total = 0;
        for (String s : list) {
            total += s.length();
        }
        return total;  // 2 + 5 + 1 = 8
    }

    static int testEmptyForEach() {
        ArrayList<String> list = new ArrayList<>();
        int count = 0;
        for (String s : list) {
            count++;
        }
        return count;  // expect 0
    }

    static int testSingleElement() {
        ArrayList<String> list = new ArrayList<>();
        list.add("only");
        for (String s : list) {
            return s.length();  // "only" = 4
        }
        return -1;
    }

    static int testAddReturnsTrue() {
        ArrayList<String> list = new ArrayList<>();
        boolean result = list.add("x");
        if (result) return 1;
        return 0;
    }
}
