import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.atomic.AtomicInteger;

public final class CyclicBarrierActionTest {
    static final class Worker implements Runnable {
        private final CyclicBarrier barrier;
        private final AtomicInteger failures;

        Worker(CyclicBarrier barrier, AtomicInteger failures) {
            this.barrier = barrier;
            this.failures = failures;
        }

        @Override
        public void run() {
            try {
                for (int i = 0; i < 4; i++) {
                    barrier.await();
                }
            } catch (Exception ignored) {
                failures.incrementAndGet();
            }
        }
    }

    public static int actionRunsOncePerGeneration() throws Exception {
        final AtomicInteger counter = new AtomicInteger(0);
        AtomicInteger failures = new AtomicInteger(0);
        CyclicBarrier barrier = new CyclicBarrier(3, new Runnable() {
            @Override
            public void run() {
                counter.incrementAndGet();
            }
        });
        Thread[] threads = new Thread[3];
        for (int i = 0; i < threads.length; i++) {
            threads[i] = new Thread(new Worker(barrier, failures));
            threads[i].start();
        }
        for (int i = 0; i < threads.length; i++) {
            threads[i].join();
        }
        if (failures.get() != 0) return -1;
        return counter.get();
    }
}
