import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase97Test {

    // ---- Sealed-like hierarchy with switch ----
    interface Shape { int area(); }
    record Circle(int r) implements Shape { public int area() { return r * r; } }
    record Rect(int w, int h) implements Shape { public int area() { return w * h; } }

    public static int testRecordShapes() {
        List<Shape> shapes = List.of(new Circle(3), new Rect(4, 5), new Circle(2));
        return shapes.stream().mapToInt(Shape::area).sum(); // 9 + 20 + 4 = 33
    }

    // ---- Map.compute ----
    public static int testMapCompute() {
        Map<String, Integer> m = new HashMap<>();
        m.put("count", 0);
        for (int i = 0; i < 5; i++) {
            m.compute("count", (k, v) -> v == null ? 1 : v + 1);
        }
        return m.get("count"); // 5
    }

    // ---- Stream.takeWhile (Java 9+) ----
    public static int testStreamTakeWhile() {
        return (int) Stream.of(1, 2, 3, 4, 5, 1, 2)
            .takeWhile(n -> n < 4)
            .count(); // takes 1,2,3 = 3
    }

    // ---- Stream.dropWhile (Java 9+) ----
    public static int testStreamDropWhile() {
        return Stream.of(1, 2, 3, 4, 5)
            .dropWhile(n -> n < 3)
            .mapToInt(Integer::intValue)
            .sum(); // 3+4+5 = 12
    }

    // ---- Collections.nCopies ----
    public static int testNCopies() {
        List<String> copies = Collections.nCopies(4, "x");
        return copies.size() + copies.get(0).length(); // 4 + 1 = 5
    }

    // ---- String.toUpperCase / toLowerCase ----
    public static int testStringCase() {
        String s = "Hello World";
        String upper = s.toUpperCase();
        String lower = s.toLowerCase();
        // "hello world" equals lower? yes; "HELLO WORLD" equals upper? yes
        return upper.equals("HELLO WORLD") && lower.equals("hello world") ? 1 : 0;
    }

    // ---- IntStream.average ----
    public static int testIntStreamAverage() {
        double avg = IntStream.of(2, 4, 6, 8, 10).average().orElse(0.0);
        return (int) avg; // 6
    }

    // ---- TreeMap ordering ----
    public static int testTreeMapOrdering() {
        TreeMap<String, Integer> tm = new TreeMap<>();
        tm.put("banana", 2);
        tm.put("apple", 1);
        tm.put("cherry", 3);
        // TreeMap iterates in key order: apple, banana, cherry
        List<Integer> vals = new ArrayList<>(tm.values());
        return vals.get(0) + vals.get(1) + vals.get(2); // 1+2+3 = 6
    }

    // ---- Long stream operations ----
    public static int testLongStreamOps() {
        long sum = LongStream.range(1L, 6L).sum(); // 1+2+3+4+5 = 15
        return (int) sum;
    }

    // ---- String.repeat ----
    public static int testStringRepeat() {
        String s = "ab".repeat(4);
        return s.length(); // 8
    }
}
