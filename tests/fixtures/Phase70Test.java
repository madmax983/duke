import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase70Test {

    // String.join(delimiter, elements)
    public static int testStringJoin() {
        String result = String.join(", ", "a", "b", "c");
        return result.length(); // "a, b, c" = 7
    }

    // String.join with List
    public static int testStringJoinList() {
        List<String> parts = new ArrayList<>();
        parts.add("foo"); parts.add("bar"); parts.add("baz");
        String result = String.join("-", parts);
        return result.length(); // "foo-bar-baz" = 11
    }

    // Collections.unmodifiableList
    public static int testUnmodifiableList() {
        List<Integer> mutable = new ArrayList<>();
        mutable.add(1); mutable.add(2); mutable.add(3);
        List<Integer> immutable = Collections.unmodifiableList(mutable);
        return immutable.size(); // 3
    }

    // Collections.singletonList
    public static int testSingletonList() {
        List<String> single = Collections.singletonList("hello");
        return single.size() * single.get(0).length(); // 1 * 5 = 5
    }

    // List.contains
    public static int testListContains() {
        List<String> list = new ArrayList<>();
        list.add("alpha"); list.add("beta"); list.add("gamma");
        int r = 0;
        if (list.contains("beta")) r += 1;
        if (!list.contains("delta")) r += 2;
        return r; // 3
    }

    // String.format with multiple types
    public static int testStringFormatMixed() {
        String s = String.format("%s=%d", "x", 42);
        return s.length(); // "x=42" = 4
    }

    // Math.max chain
    public static int testMathMaxChain() {
        int a = Math.max(1, Math.max(2, Math.max(3, 4)));
        return a; // 4
    }

    // Integer.compare
    public static int testIntegerCompare() {
        int r = 0;
        if (Integer.compare(1, 2) < 0) r += 1;
        if (Integer.compare(2, 2) == 0) r += 2;
        if (Integer.compare(3, 2) > 0) r += 4;
        return r; // 7
    }
}
