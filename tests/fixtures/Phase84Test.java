import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase84Test {

    // ---- Generic class ----
    static class Pair<A, B> {
        private final A first;
        private final B second;
        Pair(A first, B second) { this.first = first; this.second = second; }
        A getFirst() { return first; }
        B getSecond() { return second; }
    }

    public static int testGenericClass() {
        Pair<String, Integer> p = new Pair<>("hello", 42);
        return p.getFirst().length() + p.getSecond(); // 5 + 42 = 47
    }

    // ---- Generic method ----
    static <T> List<T> repeat(T item, int n) {
        List<T> result = new ArrayList<>();
        for (int i = 0; i < n; i++) result.add(item);
        return result;
    }

    public static int testGenericMethod() {
        List<Integer> list = repeat(7, 5);
        int sum = 0;
        for (Integer v : list) sum += v;
        return sum; // 35
    }

    // ---- Functional interface ----
    @FunctionalInterface
    interface Transformer<T> {
        T transform(T input);
    }

    static <T> T applyTwice(Transformer<T> t, T val) {
        return t.transform(t.transform(val));
    }

    public static int testFunctionalInterface() {
        return applyTwice(x -> x * 3, 2); // 2*3*3 = 18
    }

    // ---- BiFunction ----
    public static int testBiFunction() {
        BiFunction<Integer, Integer, Integer> add = (a, b) -> a + b;
        return add.apply(15, 27); // 42
    }

    // ---- Function.compose/andThen ----
    public static int testFunctionCompose() {
        Function<Integer, Integer> times2 = x -> x * 2;
        Function<Integer, Integer> plus3 = x -> x + 3;
        // compose: plus3 first, then times2: times2(plus3(4)) = times2(7) = 14
        Function<Integer, Integer> composed = times2.compose(plus3);
        return composed.apply(4); // 14
    }

    // ---- Predicate ----
    public static int testPredicate() {
        Predicate<Integer> isEven = n -> n % 2 == 0;
        Predicate<Integer> isPositive = n -> n > 0;
        Predicate<Integer> isEvenAndPositive = isEven.and(isPositive);
        int count = 0;
        for (int i = -5; i <= 5; i++) {
            if (isEvenAndPositive.test(i)) count++;
        }
        return count; // 2, 4 → 2 values
    }

    // ---- Consumer ----
    public static int testConsumer() {
        int[] sum = {0};
        Consumer<Integer> accumulate = x -> sum[0] += x;
        Consumer<Integer> double_acc = accumulate.andThen(x -> sum[0] += x);
        double_acc.accept(5); // sum += 5 + 5 = 10
        return sum[0]; // 10
    }

    // ---- Supplier ----
    public static int testSupplier() {
        Supplier<List<Integer>> listFactory = ArrayList::new;
        List<Integer> list = listFactory.get();
        list.add(42);
        return list.get(0); // 42
    }

    // ---- UnaryOperator ----
    public static int testUnaryOperator() {
        UnaryOperator<String> shout = s -> s.toUpperCase() + "!";
        String result = shout.apply("hello");
        return result.length(); // "HELLO!" = 6
    }
}
