import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase91Test {

    // ---- Multi-level inheritance ----
    static abstract class Animal {
        abstract String sound();
        String describe() { return "Animal says " + sound(); }
    }
    static class Dog extends Animal {
        public String sound() { return "woof"; }
    }
    static class Puppy extends Dog {
        public String sound() { return "yip"; }
    }

    public static int testMultiLevelInheritance() {
        Animal a = new Puppy();
        String desc = a.describe(); // "Animal says yip"
        return desc.length(); // 17
    }

    // ---- Interface with default + override ----
    interface Greeter {
        String greet(String name);
        default String shout(String name) { return greet(name).toUpperCase(); }
    }
    static class FormalGreeter implements Greeter {
        public String greet(String name) { return "Hello, " + name + "!"; }
    }
    static class CasualGreeter implements Greeter {
        public String greet(String name) { return "Hey " + name; }
        public String shout(String name) { return greet(name) + "!!!"; }
    }

    public static int testInterfaceDefaultOverride() {
        Greeter formal = new FormalGreeter();
        Greeter casual = new CasualGreeter();
        int len1 = formal.shout("Bob").length();   // "HELLO, BOB!" = 11
        int len2 = casual.shout("Bob").length();   // "Hey Bob!!!" = 10
        return len1 + len2; // 21
    }

    // ---- Integer overflow wrap ----
    public static int testIntegerOverflow() {
        int x = Integer.MAX_VALUE;
        int y = x + 1; // wraps to Integer.MIN_VALUE
        return y == Integer.MIN_VALUE ? 1 : 0; // 1
    }

    // ---- Long arithmetic ----
    public static int testLongArithmetic() {
        long a = 1_000_000_000L;
        long b = 3L;
        long result = a * b; // 3_000_000_000 (overflows int)
        return (int)(result / 1_000_000_000L); // 3
    }

    // ---- Nested class accesses outer static ----
    static int outerVal = 42;
    static class Inner {
        static int getOuter() { return outerVal; }
    }

    public static int testNestedClassAccessesOuter() {
        return Inner.getOuter(); // 42
    }

    // ---- Stream.distinct ----
    public static int testStreamDistinct() {
        List<Integer> list = Arrays.asList(1, 2, 2, 3, 3, 3, 4);
        return (int) list.stream().distinct().count(); // 4
    }

    // ---- Stream.limit ----
    public static int testStreamLimit() {
        return (int) IntStream.iterate(0, n -> n + 1)
            .limit(10)
            .sum(); // 0+1+...+9 = 45
    }

    // ---- Map.entrySet iteration ----
    public static int testMapEntrySet() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 1);
        m.put("b", 2);
        m.put("c", 3);
        int sum = 0;
        for (Map.Entry<String, Integer> entry : m.entrySet()) {
            sum += entry.getValue();
        }
        return sum; // 6
    }

    // ---- Recursive fibonacci ----
    static int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }

    public static int testFibonacci() {
        return fib(10); // 55
    }

    // ---- List.subList ----
    public static int testListSubList() {
        List<Integer> list = new ArrayList<>(Arrays.asList(10, 20, 30, 40, 50));
        List<Integer> sub = list.subList(1, 4); // [20, 30, 40]
        return sub.stream().mapToInt(Integer::intValue).sum(); // 90
    }
}
