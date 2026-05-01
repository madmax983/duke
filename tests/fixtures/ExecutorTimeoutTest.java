import java.util.concurrent.Callable;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

public final class ExecutorTimeoutTest {
    public static void run() throws Exception {
        ExecutorService executor = Executors.newSingleThreadExecutor();
        Future<Integer> future = executor.submit(new Callable<Integer>() {
            @Override
            public Integer call() throws Exception {
                Thread.sleep(500L);
                return Integer.valueOf(1);
            }
        });

        try {
            future.get(50L, TimeUnit.MILLISECONDS);
            System.out.println("miss");
        } catch (TimeoutException expected) {
            System.out.println("timeout");
        } finally {
            executor.shutdown();
            executor.awaitTermination(1L, TimeUnit.SECONDS);
        }
    }
}
