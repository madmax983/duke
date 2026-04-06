import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase85Test {

    // ---- Method references: instance bound ----
    public static int testBoundMethodRef() {
        String s = "Hello World";
        Supplier<Integer> len = s::length;
        return len.get(); // 11
    }

    public static int testBoundMethodRefOnArg() {
        List<String> words = List.of("apple", "banana", "cherry");
        return (int) words.stream()
            .map(String::toUpperCase)
            .filter(w -> w.startsWith("B"))
            .count(); // 1
    }

    // ---- Method references: static ----
    public static int testStaticMethodRef() {
        List<String> strs = List.of("3", "1", "4", "1", "5");
        int sum = strs.stream()
            .mapToInt(Integer::parseInt)
            .sum();
        return sum; // 14
    }

    // ---- Method references: instance unbound ----
    public static int testUnboundMethodRef() {
        List<String> words = List.of("hi", "hello", "hey");
        return words.stream()
            .mapToInt(String::length)
            .sum(); // 2+5+3 = 10
    }

    // ---- Stream.toList() (Java 16+) ----
    public static int testStreamToList() {
        List<Integer> result = List.of(1, 2, 3, 4, 5).stream()
            .filter(n -> n % 2 == 0)
            .collect(Collectors.toList());
        return result.size(); // 2 (2 and 4)
    }

    // ---- Nested lambdas ----
    public static int testNestedLambda() {
        Function<Integer, Function<Integer, Integer>> add = a -> b -> a + b;
        return add.apply(10).apply(32); // 42
    }

    // ---- Abstract class ----
    static abstract class Shape {
        abstract int area();
        int doubleArea() { return area() * 2; }
    }

    static class Circle extends Shape {
        final int r;
        Circle(int r) { this.r = r; }
        public int area() { return r * r; } // simplified (no pi)
    }

    static class Rectangle extends Shape {
        final int w, h;
        Rectangle(int w, int h) { this.w = w; this.h = h; }
        public int area() { return w * h; }
    }

    public static int testAbstractClass() {
        Shape[] shapes = { new Circle(5), new Rectangle(3, 4) };
        int total = 0;
        for (Shape s : shapes) total += s.doubleArea();
        return total; // 50 + 24 = 74
    }

    // ---- Enum with methods ----
    enum Planet {
        MERCURY(3.303e+23, 2.4397e6),
        VENUS(4.869e+24, 6.0518e6),
        EARTH(5.976e+24, 6.37814e6);

        private final double mass;
        private final double radius;
        Planet(double mass, double radius) {
            this.mass = mass;
            this.radius = radius;
        }
        double surfaceGravity() {
            final double G = 6.67300E-11;
            return G * mass / (radius * radius);
        }
    }

    public static int testEnumWithFields() {
        // Earth's surface gravity ≈ 9.8 m/s^2, so (int)(9.8) = 9
        double g = Planet.EARTH.surfaceGravity();
        return (int) g; // 9
    }

    // ---- Static initializer ----
    static class Counter {
        static int count;
        static {
            count = 100;
        }
        static int get() { return count; }
    }

    public static int testStaticInitializer() {
        return Counter.get(); // 100
    }

    // ---- String.formatted (Java 15) ----
    public static int testStringFormatted() {
        String s = "Value: %d".formatted(42);
        return s.length(); // "Value: 42".length() = 9
    }
}
