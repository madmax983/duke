import java.util.ArrayList;
import java.util.HashMap;

public class CollectionBulkTest {
    static int testAddAll() {
        ArrayList<Integer> src = new ArrayList<>();
        src.add(1);
        src.add(2);
        src.add(3);
        ArrayList<Integer> dst = new ArrayList<>();
        dst.add(0);
        dst.addAll(src);
        return dst.size();  // 4
    }

    static int testAddAllEmpty() {
        ArrayList<Integer> src = new ArrayList<>();
        ArrayList<Integer> dst = new ArrayList<>();
        dst.add(1);
        boolean modified = dst.addAll(src);
        return modified ? 1 : 0;  // 0 (empty, not modified)
    }

    static int testPutAll() {
        HashMap<String, Integer> src = new HashMap<>();
        src.put("a", 1);
        src.put("b", 2);
        HashMap<String, Integer> dst = new HashMap<>();
        dst.put("c", 3);
        dst.putAll(src);
        return dst.size();  // 3
    }
}
