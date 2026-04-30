import java.util.concurrent.atomic.AtomicInteger;

public final class AtomicContendedTest {
    static final class Worker implements Runnable {
        private final AtomicInteger counter;

        Worker(AtomicInteger counter) {
            this.counter = counter;
        }

        @Override
        public void run() {
            for (int i = 0; i < 1000; i++) {
                counter.incrementAndGet();
            }
        }
    }

    public static int twoThreadsIncrementFiftyTrials() throws Exception {
        for (int trial = 0; trial < 50; trial++) {
            AtomicInteger counter = new AtomicInteger(0);
            Thread first = new Thread(new Worker(counter));
            Thread second = new Thread(new Worker(counter));
            first.start();
            second.start();
            first.join();
            second.join();
            if (counter.get() != 2000) {
                return counter.get();
            }
        }
        return 2000;
    }
}
