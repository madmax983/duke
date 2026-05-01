import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;

public final class SemaphoreBasicTest {
    public static int basicOperations() throws Exception {
        Semaphore semaphore = new Semaphore(2);
        if (semaphore.isFair()) return -1;
        if (semaphore.availablePermits() != 2) return -2;

        semaphore.acquire();
        semaphore.acquire();
        if (semaphore.availablePermits() != 0) return -3;
        if (semaphore.tryAcquire()) return -4;

        semaphore.release();
        if (!semaphore.tryAcquire()) return -5;
        if (semaphore.availablePermits() != 0) return -6;

        semaphore.release(3);
        if (semaphore.drainPermits() != 3) return -7;
        if (semaphore.availablePermits() != 0) return -8;
        if (semaphore.tryAcquire(1L, TimeUnit.MILLISECONDS)) return -9;

        try {
            semaphore.acquire(-1);
            return -10;
        } catch (IllegalArgumentException expected) {
            return 1;
        }
    }
}
