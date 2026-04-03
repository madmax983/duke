import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase49Test {

    // ---- Comparator.thenComparing ----

    static int testComparatorThenComparing() {
        List<String> list = new ArrayList<>(Arrays.asList("banana", "apple", "cherry", "avocado"));
        // Sort by length first, then alphabetically
        list.sort(Comparator.comparingInt((String s) -> s.length())
            .thenComparing(Comparator.comparing(s -> (String) s)));
        // lengths: banana=6, apple=5, cherry=6, avocado=7
        // sorted by len: [apple(5), banana(6)/cherry(6), avocado(7)]
        // thenBy alpha: [apple, banana, cherry, avocado]
        return list.get(0).equals("apple") && list.get(1).equals("banana") ? 1 : 0;  // 1
    }

    // ---- Predicate.and ----

    static int testPredicateAnd() {
        Predicate<Integer> gt2 = x -> x.intValue() > 2;
        Predicate<Integer> lt5 = x -> x.intValue() < 5;
        Predicate<Integer> both = gt2.and(lt5);
        List<Integer> list = Arrays.asList(
            Integer.valueOf(1), Integer.valueOf(3), Integer.valueOf(5));
        long count = list.stream()
            .filter(x -> both.test((Integer) x))
            .count();
        return (int) count;  // only 3 passes both
    }

    // ---- Predicate.or ----

    static int testPredicateOr() {
        Predicate<Integer> lt2 = x -> x.intValue() < 2;
        Predicate<Integer> gt4 = x -> x.intValue() > 4;
        Predicate<Integer> either = lt2.or(gt4);
        List<Integer> list = Arrays.asList(
            Integer.valueOf(1), Integer.valueOf(3), Integer.valueOf(5));
        long count = list.stream()
            .filter(x -> either.test((Integer) x))
            .count();
        return (int) count;  // 1 and 5 pass
    }

    // ---- Predicate.negate ----

    static int testPredicateNegate() {
        Predicate<Integer> isEven = x -> x.intValue() % 2 == 0;
        Predicate<Integer> isOdd = isEven.negate();
        List<Integer> list = Arrays.asList(
            Integer.valueOf(1), Integer.valueOf(2), Integer.valueOf(3),
            Integer.valueOf(4), Integer.valueOf(5));
        long count = list.stream().filter(x -> isOdd.test((Integer) x)).count();
        return (int) count;  // 3 odd numbers
    }

    // ---- Function.andThen ----

    static int testFunctionAndThen() {
        Function<String, Integer> len = s -> Integer.valueOf(((String) s).length());
        Function<Integer, Integer> times2 = x -> Integer.valueOf(x.intValue() * 2);
        Function<String, Integer> lenTimes2 = len.andThen(times2);
        return lenTimes2.apply("hello").intValue();  // 5*2 = 10
    }

    // ---- Function.compose ----

    static int testFunctionCompose() {
        Function<Integer, Integer> times2 = x -> Integer.valueOf(x.intValue() * 2);
        Function<Integer, Integer> plus3 = x -> Integer.valueOf(x.intValue() + 3);
        // compose: f.compose(g) = f(g(x))
        Function<Integer, Integer> composed = times2.compose(plus3);
        return composed.apply(Integer.valueOf(4)).intValue();  // times2(plus3(4)) = times2(7) = 14
    }

    // ---- Stream.mapToLong ----

    static int testStreamMapToLong() {
        List<String> words = Arrays.asList("hello", "world", "java");
        long sum = words.stream()
            .mapToLong(s -> (long) ((String) s).length())
            .sum();
        return (int) sum;  // 5+5+4 = 14
    }

    // ---- Stream.mapToDouble ----

    static int testStreamMapToDouble() {
        List<Integer> nums = Arrays.asList(
            Integer.valueOf(1), Integer.valueOf(4), Integer.valueOf(9));
        double sum = nums.stream()
            .mapToDouble(x -> Math.sqrt((double) ((Integer) x).intValue()))
            .sum();
        return (int) Math.round(sum);  // 1+2+3 = 6
    }

    // ---- Collections.sort with Comparator ----

    static int testCollectionsSortComparator() {
        List<String> list = new ArrayList<>(Arrays.asList("banana", "apple", "cherry"));
        Collections.sort(list, (a, b) -> ((String) a).compareTo((String) b));
        return list.get(0).equals("apple") ? 1 : 0;  // 1
    }

    // ---- Integer.sum / min / max static ----

    static int testIntegerSum() {
        return Integer.sum(15, 27);  // 42
    }

    static int testIntegerMinMax() {
        return Integer.min(5, 3) + Integer.max(5, 3);  // 3 + 5 = 8
    }
}
