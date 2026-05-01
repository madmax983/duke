import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicLong;

public final class ExecutorPoolTest {
    public static void run() throws Exception {
        ExecutorService executor = Executors.newFixedThreadPool(4);
        AtomicLong counter = new AtomicLong(0L);

        for (int i = 0; i < 1000; i++) {
            executor.submit(new Runnable() {
                @Override
                public void run() {
                    counter.incrementAndGet();
                }
            });
        }

        executor.shutdown();
        executor.awaitTermination(2L, TimeUnit.SECONDS);
        System.out.println(counter.get());
    }
}
