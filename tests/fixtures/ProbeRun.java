import java.util.*;
import java.util.stream.*;

public class ProbeRun {
    record Point(int x, int y) {}

    public static int testStreamCollect() {
        List<Integer> src = new ArrayList<>();
        src.add(1); src.add(2); src.add(3); src.add(4); src.add(5);
        List<Integer> evens = src.stream()
            .filter(x -> x % 2 == 0)
            .collect(Collectors.toList());
        return evens.size();
    }
    public static int testStreamReduce() {
        List<Integer> nums = new ArrayList<>();
        for (int i = 1; i <= 5; i++) nums.add(i);
        return nums.stream().reduce(0, Integer::sum);
    }
    public static int testRecord() {
        Point p = new Point(3, 4);
        return p.x() + p.y();
    }
    public static int testPatternMatch() {
        Object obj = "hello";
        if (obj instanceof String s) return s.length();
        return -1;
    }
    public static int testTextBlock() {
        String tb = """
                hello
                world
                """;
        return tb.trim().length();
    }
    public static int testSwitchExpr() {
        int day = 3;
        String name = switch (day) {
            case 1 -> "Mon"; case 2 -> "Tue"; case 3 -> "Wed";
            default -> "Other";
        };
        return name.length();
    }
}
