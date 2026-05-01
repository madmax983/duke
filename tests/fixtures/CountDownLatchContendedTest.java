import java.util.concurrent.CountDownLatch;
import java.util.concurrent.atomic.AtomicInteger;

public final class CountDownLatchContendedTest {
    static final class Worker implements Runnable {
        private final CountDownLatch latch;
        private final AtomicInteger completed;
        private final long delayMillis;

        Worker(CountDownLatch latch, AtomicInteger completed, long delayMillis) {
            this.latch = latch;
            this.completed = completed;
            this.delayMillis = delayMillis;
        }

        @Override
        public void run() {
            try {
                Thread.sleep(delayMillis);
                completed.incrementAndGet();
                latch.countDown();
            } catch (Exception ignored) {
                completed.set(-1000);
            }
        }
    }

    public static int fiveWorkersCompleteTwentyFiveTrials() throws Exception {
        int total = 0;
        for (int trial = 0; trial < 25; trial++) {
            CountDownLatch latch = new CountDownLatch(5);
            AtomicInteger completed = new AtomicInteger(0);
            Thread[] threads = new Thread[5];
            for (int i = 0; i < threads.length; i++) {
                threads[i] = new Thread(new Worker(latch, completed, i + 1L));
                threads[i].start();
            }

            latch.await();
            for (int i = 0; i < threads.length; i++) {
                threads[i].join();
            }
            if (completed.get() != 5) return completed.get();
            total += completed.get();
        }
        return total;
    }
}
