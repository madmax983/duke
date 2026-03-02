public class StackOps {
    /** Returns a + b (simple baseline). */
    public static int dupAdd(int a, int b) {
        return a + b;
    }

    /**
     * Create array, store at index 0,1,2 and return sum.
     * javac uses dup for array reference preservation during stores.
     */
    public static int arrayStoreDup() {
        int[] arr = new int[3];
        arr[0] = 10;
        arr[1] = 20;
        arr[2] = 30;
        return arr[0] + arr[1] + arr[2]; // 60
    }

    /**
     * Multiple array allocations and stores — exercises dup patterns.
     */
    public static int multiAssign() {
        int[] a = new int[2];
        int[] b = new int[2];
        a[0] = 1;
        a[1] = 2;
        b[0] = 3;
        b[1] = 4;
        return a[0] + a[1] + b[0] + b[1]; // 10
    }
}
