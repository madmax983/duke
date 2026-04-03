import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase53Test {

    // ---- IntStream.findFirst ----

    static int testIntStreamFindFirst() {
        OptionalInt result = IntStream.of(10, 20, 30).findFirst();
        return result.getAsInt();  // 10
    }

    static int testIntStreamFindFirstEmpty() {
        OptionalInt result = IntStream.of().findFirst();
        return result.isPresent() ? 1 : 0;  // 0
    }

    // ---- IntStream.anyMatch / allMatch / noneMatch ----

    static int testIntStreamAnyMatch() {
        boolean result = IntStream.of(1, 2, 3, 4, 5)
            .anyMatch(n -> n > 4);
        return result ? 1 : 0;  // 1
    }

    static int testIntStreamAnyMatchFail() {
        boolean result = IntStream.of(1, 2, 3)
            .anyMatch(n -> n > 10);
        return result ? 1 : 0;  // 0
    }

    static int testIntStreamAllMatch() {
        boolean result = IntStream.of(2, 4, 6, 8)
            .allMatch(n -> n % 2 == 0);
        return result ? 1 : 0;  // 1
    }

    static int testIntStreamAllMatchFail() {
        boolean result = IntStream.of(2, 3, 4)
            .allMatch(n -> n % 2 == 0);
        return result ? 1 : 0;  // 0
    }

    static int testIntStreamNoneMatch() {
        boolean result = IntStream.of(1, 3, 5, 7)
            .noneMatch(n -> n % 2 == 0);
        return result ? 1 : 0;  // 1
    }

    static int testIntStreamNoneMatchFail() {
        boolean result = IntStream.of(1, 2, 3)
            .noneMatch(n -> n % 2 == 0);
        return result ? 1 : 0;  // 0
    }

    // ---- LongStream.findFirst ----

    static int testLongStreamFindFirst() {
        OptionalLong result = LongStream.of(100L, 200L, 300L).findFirst();
        return (int) result.getAsLong();  // 100
    }

    // ---- LongStream.anyMatch / allMatch / noneMatch ----

    static int testLongStreamAnyMatch() {
        boolean result = LongStream.of(1L, 2L, 3L, 100L)
            .anyMatch(n -> n > 50L);
        return result ? 1 : 0;  // 1
    }

    static int testLongStreamAllMatch() {
        boolean result = LongStream.of(2L, 4L, 6L)
            .allMatch(n -> n % 2L == 0L);
        return result ? 1 : 0;  // 1
    }

    static int testLongStreamNoneMatch() {
        boolean result = LongStream.of(1L, 3L, 5L)
            .noneMatch(n -> n % 2L == 0L);
        return result ? 1 : 0;  // 1
    }

    // ---- IntStream.mapToLong ----

    static int testIntStreamMapToLong() {
        long sum = IntStream.of(1, 2, 3, 4, 5)
            .mapToLong(n -> (long) n * 1000L)
            .sum();
        return (int)(sum / 1000L);  // 1+2+3+4+5 = 15
    }

    // ---- Comparator.comparingLong ----

    static int testComparatorComparingLong() {
        List<String> words = new ArrayList<>(Arrays.asList("hi", "hello", "hey", "howdy"));
        // Sort by string hashCode (long-valued key to exercise comparingLong)
        words.sort(Comparator.comparingLong(s -> (long) ((String) s).length()));
        // "hi" and "hey" len 2 and 3; "hello" 5, "howdy" 5
        return words.get(0).length();  // 2 ("hi")
    }

    // ---- Optional.or (Java 9) ----

    static int testOptionalOr() {
        Optional<String> empty = Optional.empty();
        Optional<String> result = empty.or(() -> Optional.of("fallback"));
        return result.get().length();  // 8
    }

    static int testOptionalOrPresent() {
        Optional<String> opt = Optional.of("hello");
        Optional<String> result = opt.or(() -> Optional.of("fallback"));
        return result.get().length();  // 5 — original not replaced
    }

    // ---- Optional.ifPresentOrElse (Java 9) ----

    static int testOptionalIfPresentOrElse() {
        int[] counter = {0};
        Optional<String> opt = Optional.of("hello");
        opt.ifPresentOrElse(
            s -> { counter[0] = ((String) s).length(); },
            () -> { counter[0] = -1; }
        );
        return counter[0];  // 5
    }

    static int testOptionalIfPresentOrElseEmpty() {
        int[] counter = {0};
        Optional<String> opt = Optional.empty();
        opt.ifPresentOrElse(
            s -> { counter[0] = 1; },
            () -> { counter[0] = 99; }
        );
        return counter[0];  // 99
    }

    // ---- Collectors.toUnmodifiableList (Java 10) ----

    static int testCollectorsToUnmodifiableList() {
        List<String> result = Stream.of("a", "b", "c")
            .collect(Collectors.toUnmodifiableList());
        return result.size();  // 3
    }

    static int testCollectorsToUnmodifiableListContents() {
        List<Integer> result = Stream.of(
                Integer.valueOf(10), Integer.valueOf(20), Integer.valueOf(30))
            .collect(Collectors.toUnmodifiableList());
        int sum = 0;
        for (int i = 0; i < result.size(); i++) {
            sum += ((Integer) result.get(i)).intValue();
        }
        return sum;  // 60
    }

    // ---- Collectors.toUnmodifiableSet (Java 10) ----

    static int testCollectorsToUnmodifiableSet() {
        Set<String> result = Stream.of("a", "b", "a", "c")
            .collect(Collectors.toUnmodifiableSet());
        return result.size();  // 3 (deduplicated)
    }
}
