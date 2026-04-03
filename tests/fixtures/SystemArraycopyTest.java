public class SystemArraycopyTest {
    static int testIntArrayCopy() {
        int[] src = {10, 20, 30, 40, 50};
        int[] dst = new int[5];
        System.arraycopy(src, 0, dst, 0, 5);
        return dst[2];  // 30
    }

    static int testPartialCopy() {
        int[] src = {1, 2, 3, 4, 5};
        int[] dst = {0, 0, 0, 0, 0};
        System.arraycopy(src, 1, dst, 2, 3);  // copy src[1..3] to dst[2..4]
        return dst[2] + dst[4];  // 2 + 4 = 6
    }

    static int testStringArrayCopy() {
        String[] src = {"a", "b", "c"};
        String[] dst = new String[3];
        System.arraycopy(src, 0, dst, 0, 3);
        return dst[1].length();  // "b".length() = 1
    }

    static int testOverlapSafe() {
        int[] arr = {1, 2, 3, 4, 5};
        // Shift right by 1 (non-overlapping src and dst arrays)
        int[] arr2 = new int[5];
        System.arraycopy(arr, 0, arr2, 1, 4);
        return arr2[4];  // arr[3] = 4
    }
}
