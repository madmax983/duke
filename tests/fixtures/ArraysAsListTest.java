import java.util.Arrays;
import java.util.List;

public class ArraysAsListTest {
    static int testSize() {
        List list = Arrays.asList(new Object[]{"a", "b", "c"});
        return list.size();  // 3
    }

    static int testGet() {
        List list = Arrays.asList(new Object[]{"x", "y", "z"});
        return ((String) list.get(1)).length();  // 1 ("y".length())
    }

    static int testEmptyArray() {
        List list = Arrays.asList(new Object[]{});
        return list.size();  // 0
    }
}
