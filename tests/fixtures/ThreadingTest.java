public final class ThreadingTest {
    private static final long WORKER_SLEEP_MS = 60L;

    static final class Worker implements Runnable {
        private final int id;

        Worker(int id) {
            this.id = id;
        }

        @Override
        public void run() {
            System.out.println(id);
            try {
                Thread.sleep(WORKER_SLEEP_MS);
            } catch (Exception e) {
                System.out.println(-1000 - id);
                return;
            }
            System.out.println(id + 100);
        }
    }

    static final class DerivedThread extends Thread {
        private final int value;

        DerivedThread(int value) {
            this.value = value;
        }

        @Override
        public void run() {
            System.out.println(value + 200);
        }
    }

    public static int spawnAndJoinTen() throws Exception {
        Thread[] threads = new Thread[10];
        for (int i = 0; i < threads.length; i++) {
            threads[i] = new Thread(new Worker(i));
            threads[i].start();
        }
        for (Thread thread : threads) {
            thread.join();
        }
        return threads.length;
    }

    public static int subclassRunWins() throws Exception {
        Thread thread = new DerivedThread(5);
        thread.start();
        thread.join();
        return 1;
    }

    public static int fireAndForgetStillFinishes() {
        for (int i = 0; i < 3; i++) {
            new Thread(new Worker(i)).start();
        }
        return 3;
    }

    public static int selfJoinPanic() throws Exception {
        Thread.currentThread().join();
        return 1;
    }
}
