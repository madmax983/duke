import java.util.concurrent.Semaphore;
import java.util.concurrent.atomic.AtomicInteger;

public final class SemaphoreContendedTest {
    static final class Worker implements Runnable {
        private final Semaphore semaphore;
        private final AtomicInteger inside;
        private final AtomicInteger maxInside;
        private final AtomicInteger violations;

        Worker(Semaphore semaphore, AtomicInteger inside, AtomicInteger maxInside, AtomicInteger violations) {
            this.semaphore = semaphore;
            this.inside = inside;
            this.maxInside = maxInside;
            this.violations = violations;
        }

        @Override
        public void run() {
            for (int i = 0; i < 100; i++) {
                try {
                    semaphore.acquire();
                    int now = inside.incrementAndGet();
                    while (true) {
                        int previous = maxInside.get();
                        if (now <= previous || maxInside.compareAndSet(previous, now)) break;
                    }
                    if (now > 3) violations.incrementAndGet();
                    Thread.sleep(1L);
                    inside.decrementAndGet();
                    semaphore.release();
                } catch (Exception ignored) {
                    violations.incrementAndGet();
                    return;
                }
            }
        }
    }

    public static int boundedCriticalSectionTenTrials() throws Exception {
        for (int trial = 0; trial < 10; trial++) {
            Semaphore semaphore = new Semaphore(3);
            AtomicInteger inside = new AtomicInteger(0);
            AtomicInteger maxInside = new AtomicInteger(0);
            AtomicInteger violations = new AtomicInteger(0);
            Thread[] threads = new Thread[10];
            for (int i = 0; i < threads.length; i++) {
                threads[i] = new Thread(new Worker(semaphore, inside, maxInside, violations));
                threads[i].start();
            }
            for (int i = 0; i < threads.length; i++) {
                threads[i].join();
            }
            if (violations.get() != 0) return -100 - violations.get();
            if (maxInside.get() > 3) return maxInside.get();
        }
        return 3;
    }
}
