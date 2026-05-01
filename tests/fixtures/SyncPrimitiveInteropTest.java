import java.util.concurrent.CountDownLatch;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.Semaphore;
import java.util.concurrent.atomic.AtomicInteger;

public final class SyncPrimitiveInteropTest {
    static final class Worker implements Runnable {
        private final CountDownLatch gate;
        private final Semaphore pool;
        private final CyclicBarrier barrier;
        private final AtomicInteger inside;
        private final AtomicInteger maxInside;
        private final AtomicInteger done;
        private final AtomicInteger failures;

        Worker(
            CountDownLatch gate,
            Semaphore pool,
            CyclicBarrier barrier,
            AtomicInteger inside,
            AtomicInteger maxInside,
            AtomicInteger done,
            AtomicInteger failures
        ) {
            this.gate = gate;
            this.pool = pool;
            this.barrier = barrier;
            this.inside = inside;
            this.maxInside = maxInside;
            this.done = done;
            this.failures = failures;
        }

        @Override
        public void run() {
            try {
                gate.await();
                for (int round = 0; round < 5; round++) {
                    pool.acquire();
                    int now = inside.incrementAndGet();
                    while (true) {
                        int previous = maxInside.get();
                        if (now <= previous || maxInside.compareAndSet(previous, now)) break;
                    }
                    Thread.sleep(1L);
                    inside.decrementAndGet();
                    pool.release();
                    barrier.await();
                }
                done.incrementAndGet();
            } catch (Exception ignored) {
                failures.incrementAndGet();
            }
        }
    }

    public static int primitivesCoordinateTogether() throws Exception {
        CountDownLatch gate = new CountDownLatch(1);
        Semaphore pool = new Semaphore(2);
        CyclicBarrier barrier = new CyclicBarrier(4);
        AtomicInteger inside = new AtomicInteger(0);
        AtomicInteger maxInside = new AtomicInteger(0);
        AtomicInteger done = new AtomicInteger(0);
        AtomicInteger failures = new AtomicInteger(0);

        Thread[] threads = new Thread[4];
        for (int i = 0; i < threads.length; i++) {
            threads[i] = new Thread(new Worker(gate, pool, barrier, inside, maxInside, done, failures));
            threads[i].start();
        }
        gate.countDown();
        for (int i = 0; i < threads.length; i++) {
            threads[i].join();
        }
        if (failures.get() != 0) return -1;
        if (done.get() != 4) return -2;
        if (maxInside.get() > 2) return maxInside.get();
        return 1;
    }
}
