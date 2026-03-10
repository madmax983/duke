public class GcStressTest {
    // Allocates many short-lived objects in a loop.
    // GC must fire and reclaim them, keeping heap bounded.
    public static int run() {
        int sum = 0;
        for (int i = 0; i < 2000; i++) {
            int[] arr = new int[4];
            arr[0] = i;
            sum += arr[0];
        }
        return sum; // 0+1+2+...+1999 = 1999000
    }
}
