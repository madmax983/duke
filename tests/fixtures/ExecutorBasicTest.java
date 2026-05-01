import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

public final class ExecutorBasicTest {
    public static void run() throws Exception {
        ExecutorService executor = Executors.newSingleThreadExecutor();
        AtomicInteger counter = new AtomicInteger(0);

        Future<?> future = executor.submit(new Runnable() {
            @Override
            public void run() {
                counter.incrementAndGet();
            }
        });
        future.get();
        executor.shutdown();
        executor.awaitTermination(1L, TimeUnit.SECONDS);
        System.out.println(counter.get());
    }
}
