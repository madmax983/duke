import java.util.concurrent.CancellationException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;

public final class ExecutorCancelTest {
    public static void run() throws Exception {
        ExecutorService executor = Executors.newSingleThreadExecutor();
        executor.submit(new Runnable() {
            @Override
            public void run() {
                try {
                    Thread.sleep(200L);
                } catch (Exception ignored) {
                }
            }
        });

        Future<?> cancelled = executor.submit(new Runnable() {
            @Override
            public void run() {
                System.out.println("ran");
            }
        });

        if (!cancelled.cancel(true) || !cancelled.isCancelled()) {
            System.out.println("miss");
        } else {
            try {
                cancelled.get();
                System.out.println("miss");
            } catch (CancellationException expected) {
                System.out.println("cancelled");
            }
        }

        executor.shutdown();
        executor.awaitTermination(1L, TimeUnit.SECONDS);
    }
}
