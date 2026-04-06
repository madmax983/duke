import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase100Test {

    // ---- Functional interface composition ----
    public static int testFunctionCompose() {
        Function<Integer, Integer> doubler = x -> x * 2;
        Function<Integer, Integer> adder = x -> x + 3;
        Function<Integer, Integer> composed = doubler.compose(adder); // adder then doubler
        Function<Integer, Integer> andThen = doubler.andThen(adder);  // doubler then adder
        return composed.apply(5) + andThen.apply(5); // (5+3)*2 + (5*2)+3 = 16 + 13 = 29
    }

    // ---- Predicate composition ----
    public static int testPredicateCompose() {
        Predicate<Integer> isEven = n -> n % 2 == 0;
        Predicate<Integer> isPositive = n -> n > 0;
        Predicate<Integer> both = isEven.and(isPositive);
        Predicate<Integer> either = isEven.or(isPositive);
        Predicate<Integer> notEven = isEven.negate();
        List<Integer> nums = Arrays.asList(-4, -1, 0, 2, 3, 6);
        int bothCount = (int) nums.stream().filter(both).count();   // 2 (2,6) = 2
        int eitherCount = (int) nums.stream().filter(either).count(); // 5 (-4,0,2,3,6) = 5
        int notCount = (int) nums.stream().filter(notEven).count(); // 3 (-1,3) wait...
        // notEven: not divisible by 2: -1, 3 = 2. Wait let me count:
        // -4 is even, -1 is odd (notEven=true), 0 is even, 2 is even, 3 is odd (true), 6 is even
        // notEven count = 2
        return bothCount + eitherCount + notCount; // 2 + 5 + 2 = 9
    }

    // ---- Consumer chaining ----
    public static int testConsumerAndThen() {
        int[] result = {0};
        Consumer<Integer> add = n -> result[0] += n;
        Consumer<Integer> multiply = n -> result[0] *= n;
        Consumer<Integer> combined = add.andThen(multiply);
        combined.accept(3); // first add: 0+3=3, then multiply: 3*3=9
        return result[0]; // 9
    }

    // ---- Supplier composition (via wrapper) ----
    public static int testSupplierGet() {
        Supplier<String> greeting = () -> "Hello, World!";
        return greeting.get().length(); // 13
    }

    // ---- BiFunction ----
    public static int testBiFunction() {
        BiFunction<String, Integer, String> repeat = (s, n) -> s.repeat(n);
        String result = repeat.apply("ab", 3);
        return result.length(); // "ababab" = 6
    }

    // ---- Map.forEach with BiConsumer ----
    public static int testMapForEachBiConsumer() {
        Map<String, Integer> m = new HashMap<>();
        m.put("a", 10); m.put("b", 20); m.put("c", 30);
        int[] max = {0};
        m.forEach((k, v) -> { if (v > max[0]) max[0] = v; });
        return max[0]; // 30
    }

    // ---- Collectors.summarizingInt ----
    public static int testCollectorsSummarizingInt() {
        List<Integer> nums = Arrays.asList(1, 2, 3, 4, 5);
        IntSummaryStatistics stats = nums.stream()
            .collect(Collectors.summarizingInt(Integer::intValue));
        return (int)(stats.getSum() + stats.getMin() + stats.getMax());
        // 15 + 1 + 5 = 21
    }

    // ---- Stream.mapToLong().sum() ----
    public static int testStreamMapToLong() {
        long sum = Stream.of("abc", "de", "fghi", "j")
            .mapToLong(String::length)
            .sum(); // 3+2+4+1 = 10
        return (int) sum;
    }

    // ---- Collections.unmodifiableList ----
    public static int testUnmodifiableList() {
        List<String> mutable = new ArrayList<>(Arrays.asList("a", "b", "c"));
        List<String> readonly = Collections.unmodifiableList(mutable);
        int result = 0;
        result += readonly.size(); // 3
        try {
            readonly.add("d");
        } catch (UnsupportedOperationException e) {
            result += 10; // 13
        }
        return result; // 13
    }

    // ---- IntStream.sum on method ref ----
    public static int testIntStreamMethodRef() {
        List<String> words = Arrays.asList("hello", "world", "java", "streams");
        return words.stream()
            .mapToInt(String::length)
            .sum(); // 5+5+4+7 = 21
    }
}
