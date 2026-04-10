import java.util.*;
import java.util.stream.*;

public class Phase78Test {

    // NumberFormatException catch
    public static int testNumberFormatException() {
        try {
            int n = Integer.parseInt("abc");
            return n;
        } catch (NumberFormatException e) {
            return 42;
        }
    }

    // ArrayIndexOutOfBoundsException
    public static int testArrayIndexOutOfBounds() {
        try {
            int[] arr = {1, 2, 3};
            return arr[5];
        } catch (ArrayIndexOutOfBoundsException e) {
            return 99;
        }
    }

    // NullPointerException catch
    public static int testNullPointerException() {
        try {
            String s = null;
            return s.length();
        } catch (NullPointerException e) {
            return 7;
        }
    }

    // ClassCastException
    public static int testClassCastException() {
        try {
            Object o = "hello";
            Integer n = (Integer) o;
            return n;
        } catch (ClassCastException e) {
            return 55;
        }
    }

    // StackOverflowError catch
    public static int testStackOverflow() {
        try {
            return infiniteRecursion(0);
        } catch (StackOverflowError e) {
            return 1;
        }
    }
    static int infiniteRecursion(int n) {
        return infiniteRecursion(n + 1);
    }

    // Finally block always runs
    public static int testFinallyRuns() {
        int[] result = {0};
        try {
            result[0] += 1;
            throw new RuntimeException("test");
        } catch (RuntimeException e) {
            result[0] += 10;
        } finally {
            result[0] += 100;
        }
        return result[0]; // 1 + 10 + 100 = 111
    }

    // Multi-catch
    public static int testMultiCatch() {
        int sum = 0;
        for (String s : new String[]{"1", "x", "2", null}) {
            try {
                sum += Integer.parseInt(s);
            } catch (NumberFormatException | NullPointerException e) {
                sum += 0;
            }
        }
        return sum; // 1 + 0 + 2 + 0 = 3
    }

    // Re-throw
    public static int testRethrow() {
        try {
            try {
                throw new IllegalArgumentException("inner");
            } catch (IllegalArgumentException e) {
                throw new RuntimeException("outer", e);
            }
        } catch (RuntimeException e) {
            return e.getMessage().length(); // "outer" = 5
        }

    }
}
