import java.util.List;
import java.util.Optional;
import java.util.stream.Stream;
import java.util.stream.Collectors;

public class StreamExtTest {
    static int testSorted() {
        List<String> result = Stream.of("c", "a", "b")
            .sorted()
            .collect(Collectors.toList());
        return result.get(0).equals("a") ? 1 : 0;  // 1
    }

    static int testAnyMatch() {
        boolean found = Stream.of("apple", "banana", "cherry")
            .anyMatch(s -> ((String)s).startsWith("b"));
        return found ? 1 : 0;  // 1
    }

    static int testAllMatch() {
        boolean all = Stream.of("hi", "hey", "hello")
            .allMatch(s -> ((String)s).startsWith("h"));
        return all ? 1 : 0;  // 1
    }

    static int testNoneMatch() {
        boolean none = Stream.of("cat", "dog", "fish")
            .noneMatch(s -> ((String)s).startsWith("z"));
        return none ? 1 : 0;  // 1
    }

    static int testFindFirst() {
        Optional opt = Stream.of("x", "y", "z").findFirst();
        return opt.isPresent() ? 1 : 0;  // 1
    }

    static int testFindFirstValue() {
        Optional opt = Stream.of("hello", "world").findFirst();
        return ((String) opt.get()).length();  // 5
    }

    static int testReduce() {
        Optional result = Stream.of(1, 2, 3, 4, 5)
            .map(n -> n)
            .reduce((a, b) -> {
                int ia = (Integer) a;
                int ib = (Integer) b;
                return ia + ib;
            });
        return (Integer) result.get();  // 15
    }

    static int testCollectorsJoining() {
        String result = Stream.of("a", "b", "c")
            .collect(Collectors.joining(", "));
        return result.length();  // 7 ("a, b, c")
    }

    static int testToList() {
        List<String> list = Stream.of("x", "y").toList();
        return list.size();  // 2
    }

    static int testStreamCount() {
        long n = Stream.of("a", "b", "c")
            .filter(s -> !((String)s).equals("b"))
            .count();
        return (int) n;  // 2
    }
}
