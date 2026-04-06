import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase88Test {

    // ---- Generic class ----
    static class Pair<A, B> {
        final A first;
        final B second;
        Pair(A first, B second) { this.first = first; this.second = second; }
        A getFirst() { return first; }
        B getSecond() { return second; }
    }

    public static int testGenericPair() {
        Pair<String, Integer> p = new Pair<>("hello", 42);
        return p.getFirst().length() + p.getSecond(); // 5 + 42 = 47
    }

    // ---- Stack via Deque ----
    public static int testStackDeque() {
        Deque<Integer> stack = new ArrayDeque<>();
        stack.push(1);
        stack.push(2);
        stack.push(3);
        return stack.pop() + stack.pop(); // 3 + 2 = 5
    }

    // ---- BitSet ----
    public static int testBitSet() {
        BitSet bs = new BitSet(16);
        bs.set(3);
        bs.set(7);
        bs.set(12);
        return bs.cardinality(); // 3
    }

    // ---- Math functions ----
    public static int testMathFunctions() {
        int a = (int) Math.min(10.0, 5.0); // 5
        int b = Math.max(3, 7); // 7
        return a + b; // 12
    }

    // ---- Character classification ----
    public static int testCharacterMethods() {
        int count = 0;
        String s = "Hello World 123";
        for (char c : s.toCharArray()) {
            if (Character.isDigit(c)) count++;
        }
        return count; // 3
    }

    // ---- String split and join ----
    public static int testStringSplitJoin() {
        String csv = "one,two,three,four";
        String[] parts = csv.split(",");
        String joined = String.join("|", parts);
        return joined.length(); // "one|two|three|four" = 18
    }

    // ---- Interface with default method ----
    interface Greeter {
        String greet(String name);
        default int greetLength(String name) {
            return greet(name).length();
        }
    }

    static class HelloGreeter implements Greeter {
        public String greet(String name) { return "Hello, " + name + "!"; }
    }

    public static int testInterfaceDefault() {
        Greeter g = new HelloGreeter();
        return g.greetLength("World"); // "Hello, World!" = 13
    }

    // ---- Static interface method ----
    interface Validator {
        boolean isValid(String s);
        static Validator nonEmpty() { return s -> !s.isEmpty(); }
    }

    public static int testStaticInterfaceMethod() {
        Validator v = Validator.nonEmpty();
        return v.isValid("hello") ? 1 : 0; // 1
    }

    // ---- Chained stream with collect ----
    public static int testStreamCollectToMap() {
        Map<String, Integer> m = List.of("a", "bb", "ccc")
            .stream()
            .collect(Collectors.toMap(Function.identity(), String::length));
        return m.get("ccc"); // 3
    }

    // ---- Exception with message ----
    public static int testExceptionMessage() {
        try {
            throw new IllegalStateException("bad state");
        } catch (IllegalStateException e) {
            return e.getMessage().length(); // "bad state" = 9
        }
    }
}
