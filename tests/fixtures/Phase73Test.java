import java.util.*;
import java.util.stream.*;

public class Phase73Test {

    // Stream.reduce with identity
    public static int testStreamReduceIdentity() {
        List<Integer> nums = new ArrayList<>();
        nums.add(1); nums.add(2); nums.add(3); nums.add(4);
        int result = nums.stream().reduce(0, Integer::sum);
        return result; // 10
    }

    // Stream.reduce without identity (Optional)
    public static int testStreamReduceOptional() {
        List<Integer> nums = new ArrayList<>();
        nums.add(5); nums.add(3); nums.add(8);
        Optional<Integer> result = nums.stream().reduce(Integer::max);
        return result.orElse(0); // 8
    }

    // Stream.min / max
    public static int testStreamMinMax() {
        List<Integer> nums = new ArrayList<>();
        nums.add(3); nums.add(1); nums.add(4); nums.add(1); nums.add(5);
        int min = nums.stream().mapToInt(x -> x).min().orElse(-1);
        int max = nums.stream().mapToInt(x -> x).max().orElse(-1);
        return min * 10 + max; // 1*10 + 5 = 15
    }

    // Collectors.joining
    public static int testCollectorsJoining() {
        List<String> words = new ArrayList<>();
        words.add("hello"); words.add("world");
        String result = words.stream().collect(Collectors.joining(", "));
        return result.length(); // "hello, world" = 12
    }

    // Collectors.joining with prefix/suffix
    public static int testCollectorsJoiningPrefixSuffix() {
        List<String> items = new ArrayList<>();
        items.add("a"); items.add("b"); items.add("c");
        String result = items.stream().collect(Collectors.joining(", ", "[", "]"));
        return result.length(); // "[a, b, c]" = 9
    }

    // Stream.findFirst (Optional)
    public static int testStreamFindFirst() {
        List<Integer> nums = new ArrayList<>();
        nums.add(10); nums.add(20); nums.add(30);
        Optional<Integer> first = nums.stream().filter(n -> n > 15).findFirst();
        return first.orElse(-1); // 20
    }

    // Optional.isPresent / get
    public static int testOptionalOperations() {
        Optional<String> present = Optional.of("hello");
        Optional<String> empty = Optional.empty();
        int r = 0;
        if (present.isPresent()) r += 1;
        if (!empty.isPresent()) r += 2;
        r += present.get().length(); // + 5
        return r; // 1 + 2 + 5 = 8
    }

    // Collectors.toSet
    public static int testCollectorsToSet() {
        List<Integer> nums = new ArrayList<>();
        nums.add(1); nums.add(2); nums.add(2); nums.add(3); nums.add(3);
        Set<Integer> unique = nums.stream().collect(Collectors.toSet());
        return unique.size(); // 3
    }
}
