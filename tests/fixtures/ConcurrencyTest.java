/**
 * Tests that multiple Java threads make concurrent progress via the
 * quantum-based time-slicing scheduler.  Each worker performs a busy
 * loop and prints markers at regular intervals; if execution were
 * serialised the markers from one worker would cluster together,
 * whereas with fair interleaving they will be mixed.
 */
public final class ConcurrencyTest {

    /** Number of busy-loop iterations each worker performs. */
    private static final int WORK_ITERATIONS = 8000;

    /** Workers print a marker every PRINT_INTERVAL iterations. */
    private static final int PRINT_INTERVAL = 2000;

    static final class BusyWorker implements Runnable {
        private final int id;

        BusyWorker(int id) {
            this.id = id;
        }

        @Override
        public void run() {
            int acc = 0;
            for (int i = 1; i <= WORK_ITERATIONS; i++) {
                acc = acc + i;
                if (i % PRINT_INTERVAL == 0) {
                    // Format: "<worker_id>:<checkpoint>"
                    System.out.println(id + ":" + (i / PRINT_INTERVAL));
                }
            }
        }
    }

    /**
     * Spawn two busy workers and join both.  Returns 2 on success.
     * The output lines are checked on the Rust side to verify that
     * markers from both workers are interleaved (not serialised).
     */
    public static int twoWorkersBusyLoop() throws Exception {
        Thread t0 = new Thread(new BusyWorker(0));
        Thread t1 = new Thread(new BusyWorker(1));
        t0.start();
        t1.start();
        t0.join();
        t1.join();
        return 2;
    }
}
