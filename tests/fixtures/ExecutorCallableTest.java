import java.util.concurrent.Callable;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

public final class ExecutorCallableTest {
    public static void run() throws Exception {
        ExecutorService executor = Executors.newFixedThreadPool(2);
        Future<Integer> future = executor.submit(new Callable<Integer>() {
            @Override
            public Integer call() {
                return Integer.valueOf(42);
            }
        });
        System.out.println(future.get());
        executor.shutdown();
        executor.awaitTermination(1L, TimeUnit.SECONDS);
    }

    public static int cachedFactoryWorks() throws Exception {
        ExecutorService executor = Executors.newCachedThreadPool();
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
        return counter.get();
    }

    public static int runnableWithResultWorks() throws Exception {
        ExecutorService executor = Executors.newSingleThreadExecutor();
        Future<Integer> future = executor.submit(new Runnable() {
            @Override
            public void run() {
            }
        }, Integer.valueOf(77));
        int result = future.get().intValue();
        executor.shutdown();
        executor.awaitTermination(1L, TimeUnit.SECONDS);
        return result;
    }

    public static int executionExceptionWrapsCause() throws Exception {
        ExecutorService executor = Executors.newSingleThreadExecutor();
        Future<Integer> future = executor.submit(new Callable<Integer>() {
            @Override
            public Integer call() {
                throw new RuntimeException("boom");
            }
        });
        try {
            future.get();
            return -1;
        } catch (ExecutionException expected) {
            Throwable cause = expected.getCause();
            if (!(cause instanceof RuntimeException)) {
                return -2;
            }
            return "boom".equals(cause.getMessage()) ? 1 : -3;
        } finally {
            executor.shutdown();
            executor.awaitTermination(1L, TimeUnit.SECONDS);
        }
    }

    public static long secondsToMillis() {
        return TimeUnit.SECONDS.toMillis(2L);
    }
}
