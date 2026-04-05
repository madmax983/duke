import java.util.*;
import java.util.stream.*;
import java.util.function.*;

public class Phase80Test {

    // String.valueOf for various types
    public static int testStringValueOfChar() {
        String s = String.valueOf('Z');
        return s.length(); // 1
    }

    public static int testStringValueOfCharArray() {
        char[] arr = {'h', 'i'};
        String s = String.valueOf(arr);
        return s.length(); // 2
    }

    // Character arithmetic and boxing
    public static int testCharArithmetic() {
        char a = 'A';
        char z = 'Z';
        return z - a; // 25
    }

    public static int testCharBoxing() {
        Character c = Character.valueOf('X');
        return c.charValue() - 'A'; // 23
    }

    // Integer.bitCount
    public static int testIntegerBitCount() {
        return Integer.bitCount(255); // 8
    }

    // Integer.highestOneBit / lowestOneBit
    public static int testIntegerHighestOneBit() {
        return Integer.highestOneBit(100); // 64
    }

    public static int testIntegerLowestOneBit() {
        return Integer.lowestOneBit(12); // 4
    }

    // Integer.numberOfLeadingZeros
    public static int testIntegerNumberOfLeadingZeros() {
        return Integer.numberOfLeadingZeros(1); // 31
    }

    // Long.bitCount
    public static int testLongBitCount() {
        return Long.bitCount(255L); // 8
    }

    // Math.min/max with long
    public static int testMathLong() {
        long a = Math.min(100L, 200L);
        long b = Math.max(100L, 200L);
        return (int)(b - a); // 100
    }

    // Iterable for-each on array-backed list
    public static int testForEachLambda() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5));
        int[] sum = {0};
        list.forEach(x -> sum[0] += x);
        return sum[0]; // 15
    }

    // Collection.removeIf
    public static int testRemoveIf() {
        List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3, 4, 5, 6));
        list.removeIf(x -> x % 2 == 0);
        return list.size(); // 3 (1,3,5 remain)
    }

    // String.chars() as IntStream counting vowels
    public static int testStringCharsCount() {
        String s = "hello world";
        return (int) s.chars().filter(c -> "aeiou".indexOf(c) >= 0).count(); // 3
    }

    // Collections.swap
    public static int testCollectionsSwap() {
        List<Integer> list = new ArrayList<>(Arrays.asList(10, 20, 30));
        Collections.swap(list, 0, 2);
        return list.get(0); // 30
    }

    // Collections.min/max
    public static int testCollectionsMinMax() {
        List<Integer> list = Arrays.asList(3, 1, 4, 1, 5, 9, 2, 6);
        int min = (Integer) Collections.min(list);
        int max = (Integer) Collections.max(list);
        return max - min; // 9 - 1 = 8
    }
}
