import java.util.ArrayList;
import java.util.List;
import java.util.stream.Stream;
import java.util.stream.Collectors;

public class StreamTest {
    static int testStreamOfCount() {
        return (int) Stream.of("a", "b", "c").count();  // 3
    }

    static int testStreamFilter() {
        List<String> result = Stream.of("a", "bb", "ccc", "d")
            .filter(s -> s.length() > 1)
            .collect(Collectors.toList());
        return result.size();  // 2
    }

    static int testStreamMap() {
        List<Integer> result = Stream.of("hello", "hi", "hey")
            .map(s -> s.length())
            .collect(Collectors.toList());
        return result.get(0);  // 5
    }

    static int testStreamForEach() {
        int[] sum = {0};
        Stream.of(1, 2, 3).forEach(n -> sum[0] += (Integer) n);
        return sum[0];  // 6
    }

    static int testCollectionStream() {
        ArrayList<String> list = new ArrayList<>();
        list.add("x");
        list.add("y");
        list.add("z");
        return (int) list.stream().count();  // 3
    }

    static int testStreamToList() {
        List<String> result = Stream.of("a", "b").collect(Collectors.toList());
        return result.size();  // 2
    }

    static int testStreamDistinct() {
        List<String> result = Stream.of("a", "b", "a", "c", "b")
            .distinct()
            .collect(Collectors.toList());
        return result.size();  // 3
    }

    static int testStreamMapToLength() {
        long count = Stream.of("hello", "world").map(s -> ((String)s).toUpperCase()).count();
        return (int) count;  // 2
    }
}
