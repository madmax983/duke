import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.atomic.AtomicInteger;

public final class CyclicBarrierBasicTest {
    static final class Worker implements Runnable {
        private final CyclicBarrier barrier;
        private final AtomicInteger firstMask;
        private final AtomicInteger secondMask;
        private final AtomicInteger failures;

        Worker(CyclicBarrier barrier, AtomicInteger firstMask, AtomicInteger secondMask, AtomicInteger failures) {
            this.barrier = barrier;
            this.firstMask = firstMask;
            this.secondMask = secondMask;
            this.failures = failures;
        }

        @Override
        public void run() {
            try {
                int first = barrier.await();
                firstMask.addAndGet(1 << first);
                int second = barrier.await();
                secondMask.addAndGet(1 << second);
            } catch (Exception ignored) {
                failures.incrementAndGet();
            }
        }
    }

    public static int twoCyclesReturnAllArrivalIndexes() throws Exception {
        CyclicBarrier barrier = new CyclicBarrier(3);
        AtomicInteger firstMask = new AtomicInteger(0);
        AtomicInteger secondMask = new AtomicInteger(0);
        AtomicInteger failures = new AtomicInteger(0);
        Thread[] threads = new Thread[3];
        for (int i = 0; i < threads.length; i++) {
            threads[i] = new Thread(new Worker(barrier, firstMask, secondMask, failures));
            threads[i].start();
        }
        for (int i = 0; i < threads.length; i++) {
            threads[i].join();
        }
        if (failures.get() != 0) return -1;
        if (firstMask.get() != 7) return firstMask.get();
        if (secondMask.get() != 7) return secondMask.get();
        if (barrier.getParties() != 3) return -2;
        if (barrier.getNumberWaiting() != 0) return -3;
        return 1;
    }
}
