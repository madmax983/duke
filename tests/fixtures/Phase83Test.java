import java.util.*;
import java.util.stream.*;

public class Phase83Test {

    // ---- interface default methods ----
    interface Greeter {
        String greet(String name);
        default String greetLoudly(String name) {
            return greet(name).toUpperCase();
        }
    }

    static class HelloGreeter implements Greeter {
        public String greet(String name) { return "Hello " + name; }
    }

    public static int testInterfaceDefaultMethod() {
        Greeter g = new HelloGreeter();
        return g.greetLoudly("world").length(); // "HELLO WORLD".length() = 11
    }

    // ---- static interface method ----
    interface MathHelper {
        static int square(int n) { return n * n; }
        default int cube(int n) { return n * n * n; }
    }

    static class MyMath implements MathHelper {}

    public static int testStaticInterfaceMethod() {
        return MathHelper.square(7); // 49
    }

    public static int testDefaultInterfaceMethod() {
        MathHelper m = new MyMath();
        return m.cube(3); // 27
    }

    // ---- enhanced switch expression (Java 14+) ----
    public static int testSwitchExpression() {
        int day = 3;
        int result = switch (day) {
            case 1, 7 -> 0;   // weekend
            case 2, 3, 4, 5, 6 -> 1; // weekday
            default -> -1;
        };
        return result; // 1
    }

    public static int testSwitchExpressionYield() {
        int x = 5;
        int result = switch (x) {
            case 1 -> 10;
            case 5 -> {
                int v = x * x;
                yield v; // 25
            }
            default -> 0;
        };
        return result; // 25
    }

    // ---- record class (Java 16+) ----
    record Point(int x, int y) {
        int sum() { return x + y; }
    }

    public static int testRecord() {
        Point p = new Point(3, 7);
        return p.x() + p.y(); // 10
    }

    public static int testRecordMethod() {
        Point p = new Point(4, 6);
        return p.sum(); // 10
    }

    // ---- sealed class / pattern matching instanceof (Java 16+) ----
    public static int testPatternMatchingInstanceof() {
        Object o = Integer.valueOf(42);
        if (o instanceof Integer i) {
            return i; // 42
        }
        return 0;
    }

    // ---- text block (Java 15+) ----
    public static int testTextBlock() {
        String s = """
                Hello
                World
                """;
        // 2 lines + newlines = "Hello\nWorld\n" = 12 chars
        return s.trim().length(); // "Hello\nWorld" = 11
    }
}
