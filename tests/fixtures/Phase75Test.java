import java.util.*;
import java.util.stream.*;

public class Phase75Test {

    // String.chars() stream operations
    public static int testStringCharsFilter() {
        String s = "Hello World";
        long upper = s.chars().filter(Character::isUpperCase).count();
        return (int) upper; // H, W = 2
    }

    // String.chars() to array sum
    public static int testStringCharsSum() {
        String s = "abc";
        int sum = s.chars().sum(); // 'a'=97, 'b'=98, 'c'=99 = 294
        return sum - 290; // 4 (to keep small return value)
    }

    // Arrays.stream(int[])
    public static int testArraysStreamInt() {
        int[] arr = {3, 1, 4, 1, 5, 9};
        return Arrays.stream(arr).sum(); // 23
    }

    // Arrays.stream(int[]).filter
    public static int testArraysStreamFilter() {
        int[] arr = {1, 2, 3, 4, 5, 6};
        return (int) Arrays.stream(arr).filter(x -> x % 2 == 0).count(); // 3
    }

    // Arrays.asList
    public static int testArraysAsList() {
        List<String> list = Arrays.asList("a", "b", "c", "d");
        return list.size(); // 4
    }

    // Collections.nCopies
    public static int testCollectionsNCopies() {
        List<String> copies = Collections.nCopies(5, "x");
        int len = 0;
        for (String s : copies) len += s.length();
        return len; // 5
    }

    // Iterable for-each with lambda
    public static int testIterableForEach() {
        List<Integer> list = new ArrayList<>();
        list.add(1); list.add(2); list.add(3);
        int[] sum = {0};
        list.forEach(x -> sum[0] += x);
        return sum[0]; // 6
    }

    // String.chars() distinct count
    public static int testStringCharsDistinct() {
        String s = "aabbcc";
        long distinct = s.chars().distinct().count();
        return (int) distinct; // 3
    }
}
