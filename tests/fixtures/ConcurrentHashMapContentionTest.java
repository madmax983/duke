import java.util.concurrent.ConcurrentHashMap;

public final class ConcurrentHashMapContentionTest {
    static final class Worker implements Runnable {
        private final ConcurrentHashMap<Integer, Integer> map;
        private final int threadId;

        Worker(ConcurrentHashMap<Integer, Integer> map, int threadId) {
            this.map = map;
            this.threadId = threadId;
        }

        @Override
        public void run() {
            int bucket = threadId % 8;
            for (int i = 0; i < 1000; i++) {
                map.merge(bucket, 1, Integer::sum);
            }
        }
    }

    public static int twoThreadsMergeFiftyTrials() throws Exception {
        for (int trial = 0; trial < 50; trial++) {
            ConcurrentHashMap<Integer, Integer> map = new ConcurrentHashMap<>();
            Thread first = new Thread(new Worker(map, 1));
            Thread second = new Thread(new Worker(map, 2));
            first.start();
            second.start();
            first.join();
            second.join();
            int sum = 0;
            for (int bucket = 0; bucket < 8; bucket++) {
                sum += map.getOrDefault(bucket, 0);
            }
            if (sum != 2000) return sum;
        }
        return 2000;
    }
}
