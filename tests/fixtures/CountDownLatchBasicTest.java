import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

public final class CountDownLatchBasicTest {
    public static int basicOperations() throws Exception {
        CountDownLatch latch = new CountDownLatch(3);
        if (latch.getCount() != 3L) return -1;

        latch.countDown();
        latch.countDown();
        if (latch.getCount() != 1L) return -2;

        latch.countDown();
        latch.await();
        if (latch.getCount() != 0L) return -3;

        latch.countDown();
        if (latch.getCount() != 0L) return -4;
        if (!latch.await(10L, TimeUnit.MILLISECONDS)) return -5;
        if (!latch.toString().contains("Count = 0")) return -6;

        try {
            new CountDownLatch(-1);
            return -7;
        } catch (IllegalArgumentException expected) {
            return 1;
        }
    }
}
