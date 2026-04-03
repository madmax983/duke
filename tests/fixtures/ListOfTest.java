import java.util.List;
import java.util.Set;
import java.util.Map;

public class ListOfTest {
    static int testListOfZero() {
        return List.of().size();  // 0
    }

    static int testListOfOne() {
        return List.of("a").size();  // 1
    }

    static int testListOfThree() {
        List list = List.of("a", "b", "c");
        return list.size();  // 3
    }

    static int testListOfGet() {
        List list = List.of("x", "y", "z");
        return ((String) list.get(1)).length();  // 1
    }

    static int testSetOfTwo() {
        Set set = Set.of("a", "b");
        return set.size();  // 2
    }

    static int testMapOfOne() {
        Map map = Map.of("key", "value");
        return ((String) map.get("key")).length();  // 5
    }

    static int testMapOfTwo() {
        Map map = Map.of("a", 1, "b", 2);
        return map.size();  // 2
    }
}
