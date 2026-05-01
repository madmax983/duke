import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.locks.ReentrantReadWriteLock;

public final class ReadWriteLockTest {
    public static int twoReadersMayOverlap() throws Exception {
        ReentrantReadWriteLock rw = new ReentrantReadWriteLock();
        AtomicInteger inside = new AtomicInteger(0);
        AtomicInteger maxInside = new AtomicInteger(0);
        AtomicInteger value = new AtomicInteger(0);
        AtomicInteger successfulReads = new AtomicInteger(0);

        rw.writeLock().lock();
        try {
            value.set(42);
        } finally {
            rw.writeLock().unlock();
        }

        Runnable reader = new Runnable() {
            @Override
            public void run() {
                rw.readLock().lock();
                try {
                    int now = inside.incrementAndGet();
                    if (now > maxInside.get()) {
                        maxInside.set(now);
                    }
                    if (value.get() == 42) {
                        successfulReads.incrementAndGet();
                    }
                    try {
                        Thread.sleep(10L);
                    } catch (Exception ignored) {
                        return;
                    }
                } finally {
                    inside.decrementAndGet();
                    rw.readLock().unlock();
                }
            }
        };

        Thread first = new Thread(reader);
        Thread second = new Thread(reader);
        first.start();
        second.start();
        first.join();
        second.join();
        return maxInside.get() == 2 && successfulReads.get() == 2 ? 1 : -1;
    }

    public static int writeLockExcludesReadersUntilUnlock() throws Exception {
        ReentrantReadWriteLock rw = new ReentrantReadWriteLock();
        AtomicInteger phase = new AtomicInteger(0);
        rw.writeLock().lock();
        Thread reader = new Thread(new Runnable() {
            @Override
            public void run() {
                phase.set(1);
                rw.readLock().lock();
                try {
                    phase.set(2);
                } finally {
                    rw.readLock().unlock();
                }
            }
        });
        reader.start();
        while (phase.get() == 0) {
            Thread.sleep(1L);
        }
        Thread.sleep(5L);
        int blocked = phase.get() == 1 ? 1 : -10;
        rw.writeLock().unlock();
        reader.join();
        return blocked + phase.get();
    }

    public static int readLockExcludesWriterTryLock() {
        ReentrantReadWriteLock rw = new ReentrantReadWriteLock();
        rw.readLock().lock();
        try {
            return rw.writeLock().tryLock() ? -1 : 1;
        } finally {
            rw.readLock().unlock();
        }
    }

    public static int writeReentrantAndDowngrade() {
        ReentrantReadWriteLock rw = new ReentrantReadWriteLock();
        rw.writeLock().lock();
        rw.writeLock().lock();
        try {
            rw.readLock().lock();
            try {
                return rw.writeLock().tryLock() ? 1 : -1;
            } finally {
                rw.readLock().unlock();
                rw.writeLock().unlock();
            }
        } finally {
            rw.writeLock().unlock();
            rw.writeLock().unlock();
        }
    }

    public static int readLockNewConditionUnsupported() {
        ReentrantReadWriteLock rw = new ReentrantReadWriteLock();
        try {
            rw.readLock().newCondition();
            return -1;
        } catch (UnsupportedOperationException expected) {
            return 1;
        }
    }

    public static int writeUnlockByNonOwnerThrows() throws Exception {
        ReentrantReadWriteLock rw = new ReentrantReadWriteLock();
        AtomicInteger result = new AtomicInteger(0);
        rw.writeLock().lock();
        Thread thief = new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    rw.writeLock().unlock();
                    result.set(-1);
                } catch (IllegalMonitorStateException expected) {
                    result.set(1);
                }
            }
        });
        thief.start();
        thief.join();
        rw.writeLock().unlock();
        return result.get();
    }
}
