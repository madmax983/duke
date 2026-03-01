public class ArrayOps {

    /** Allocate int[n], fill with 1..n, return sum.
     *  Exercises: newarray T_INT, iastore, iaload, arraylength */
    public static int sumArray(int n) {
        int[] a = new int[n];
        for (int i = 0; i < n; i++) {
            a[i] = i + 1;
        }
        int sum = 0;
        for (int i = 0; i < a.length; i++) {
            sum += a[i];
        }
        return sum;
    }

    /** Return length of a newly-allocated int[n].
     *  Exercises: newarray, arraylength */
    public static int arrayLength(int n) {
        int[] a = new int[n];
        return a.length;
    }

    /** Allocate long[n], fill with i*1_000_000, return sum.
     *  Exercises: newarray T_LONG, lastore, laload */
    public static long sumLongArray(int n) {
        long[] a = new long[n];
        for (int i = 0; i < n; i++) {
            a[i] = (long) i * 1_000_000L;
        }
        long sum = 0L;
        for (int i = 0; i < n; i++) {
            sum += a[i];
        }
        return sum;
    }

    /** Allocate double[n], fill with i * 0.5, return first element.
     *  Exercises: newarray T_DOUBLE, dastore, daload */
    public static double firstDouble(int n) {
        double[] a = new double[n];
        for (int i = 0; i < n; i++) {
            a[i] = i * 0.5;
        }
        return a[0];
    }
}
