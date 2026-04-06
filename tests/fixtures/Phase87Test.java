import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase87Test {

    // ---- Collector.joining ----
    public static int testStreamJoining() {
        String result = List.of("a", "b", "c")
            .stream()
            .collect(Collectors.joining(", "));
        return result.length(); // "a, b, c" = 7
    }

    // ---- Stream.flatMap ----
    public static int testStreamFlatMap() {
        List<List<Integer>> nested = List.of(
            List.of(1, 2),
            List.of(3, 4),
            List.of(5)
        );
        return nested.stream()
            .flatMap(Collection::stream)
            .mapToInt(Integer::intValue)
            .sum(); // 1+2+3+4+5 = 15
    }

    // ---- Stream.reduce ----
    public static int testStreamReduce() {
        return List.of(1, 2, 3, 4, 5)
            .stream()
            .reduce(0, Integer::sum); // 15
    }

    // ---- Stream.sorted with comparator ----
    public static int testStreamSortedWithComparator() {
        List<Integer> result = List.of(3, 1, 4, 1, 5).stream()
            .sorted(Comparator.reverseOrder())
            .collect(Collectors.toList());
        return result.get(0); // 5
    }

    // ---- Map.Entry iteration ----
    public static int testMapEntrySum() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 10);
        m.put("b", 20);
        m.put("c", 30);
        int sum = 0;
        for (Map.Entry<String, Integer> e : m.entrySet()) {
            sum += e.getValue();
        }
        return sum; // 60
    }

    // ---- instanceof pattern ----
    public static int testInstanceof() {
        Object o = "hello";
        if (o instanceof String s) {
            return s.length(); // 5
        }
        return 0;
    }

    // ---- switch expression ----
    public static int testSwitchExpression() {
        int day = 3;
        String name = switch (day) {
            case 1 -> "Monday";
            case 2 -> "Tuesday";
            case 3 -> "Wednesday";
            default -> "Other";
        };
        return name.length(); // "Wednesday" = 9
    }

    // ---- text block (Java 15+) ----
    public static int testTextBlock() {
        String json = """
                {"key": "value"}
                """;
        return json.trim().length(); // {"key": "value"} = 16
    }

    // ---- Multi-dimensional array ----
    public static int testMultiDimArray() {
        int[][] matrix = {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}};
        int sum = 0;
        for (int[] row : matrix) {
            for (int v : row) sum += v;
        }
        return sum; // 45
    }

    // ---- String.format with multiple types ----
    public static int testStringFormatMulti() {
        String s = String.format("%s is %d years old", "Alice", 30);
        return s.length(); // "Alice is 30 years old" = 21
    }
}
