import java.util.Arrays;

public class ArraysTest {
    static int testFillInt() {
        int[] arr = new int[4];
        Arrays.fill(arr, 7);
        return arr[0] + arr[3];  // 7 + 7 = 14
    }

    static int testCopyOfTruncate() {
        int[] arr = {10, 20, 30, 40, 50};
        int[] copy = Arrays.copyOf(arr, 3);
        return copy.length;  // 3
    }

    static int testCopyOfExtend() {
        int[] arr = {1, 2};
        int[] copy = Arrays.copyOf(arr, 5);
        return copy[4];  // 0 (zero-padded)
    }

    static int testSortInt() {
        int[] arr = {5, 2, 8, 1, 9, 3};
        Arrays.sort(arr);
        return arr[0] * 10 + arr[5];  // 1*10 + 9 = 19
    }

    static int testIntegerMaxValue() {
        if (Integer.MAX_VALUE == 2147483647) return 1;
        return 0;
    }

    static int testIntegerMinValue() {
        if (Integer.MIN_VALUE == -2147483648) return 1;
        return 0;
    }

    static int testLongMaxValue() {
        if (Long.MAX_VALUE > 0) return 1;
        return 0;
    }

    static int testDoubleMaxValue() {
        if (Double.MAX_VALUE > 0.0) return 1;
        return 0;
    }

    static int testDoubleNaN() {
        double n = Double.NaN;
        if (Double.isNaN(n)) return 1;
        return 0;
    }
}
