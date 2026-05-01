import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.locks.ReentrantLock;

public final class ReentrantLockBasicTest {
    public static int reentrantHoldCountAndStatus() {
        ReentrantLock lock = new ReentrantLock(true);
        if (lock.isLocked()) return -1;
        if (lock.isHeldByCurrentThread()) return -2;
        if (lock.getHoldCount() != 0) return -3;

        lock.lock();
        try {
            if (!lock.isLocked()) return -4;
            if (!lock.isHeldByCurrentThread()) return -5;
            if (lock.getHoldCount() != 1) return -6;

            lock.lock();
            try {
                if (lock.getHoldCount() != 2) return -7;
            } finally {
                lock.unlock();
            }

            return lock.getHoldCount() == 1 ? 1 : -8;
        } finally {
            lock.unlock();
        }
    }

    public static int tryLockFailsWhileAnotherThreadHolds() throws Exception {
        ReentrantLock lock = new ReentrantLock();
        AtomicInteger phase = new AtomicInteger(0);

        Thread holder = new Thread(new Runnable() {
            @Override
            public void run() {
                lock.lock();
                try {
                    phase.set(1);
                    while (phase.get() == 1) {
                        try {
                            Thread.sleep(1L);
                        } catch (Exception ignored) {
                            return;
                        }
                    }
                } finally {
                    lock.unlock();
                }
            }
        });

        holder.start();
        while (phase.get() == 0) {
            Thread.sleep(1L);
        }

        boolean blocked = !lock.tryLock();
        phase.set(2);
        holder.join();

        lock.lock();
        try {
            return blocked && phase.get() == 2 ? 1 : -1;
        } finally {
            lock.unlock();
        }
    }

    public static int unlockByNonOwnerThrows() throws Exception {
        ReentrantLock lock = new ReentrantLock();
        AtomicInteger result = new AtomicInteger(0);
        lock.lock();
        try {
            Thread thief = new Thread(new Runnable() {
                @Override
                public void run() {
                    try {
                        lock.unlock();
                        result.set(-1);
                    } catch (IllegalMonitorStateException expected) {
                        result.set(1);
                    }
                }
            });
            thief.start();
            thief.join();
            return result.get();
        } finally {
            lock.unlock();
        }
    }
}
