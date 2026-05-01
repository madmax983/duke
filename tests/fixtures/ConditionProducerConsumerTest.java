import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.locks.Condition;
import java.util.concurrent.locks.ReentrantLock;

public final class ConditionProducerConsumerTest {
    static final class Buffer {
        final ReentrantLock lock = new ReentrantLock();
        final Condition notEmpty = lock.newCondition();
        final Condition notFull = lock.newCondition();
        boolean full;
        int slot;
        int consumed;
    }

    static final class Producer implements Runnable {
        private final Buffer buffer;

        Producer(Buffer buffer) {
            this.buffer = buffer;
        }

        @Override
        public void run() {
            for (int i = 1; i <= 5; i++) {
                buffer.lock.lock();
                try {
                    while (buffer.full) {
                        buffer.notFull.awaitNanos(1000000L);
                    }
                    buffer.slot = i;
                    buffer.full = true;
                    buffer.notEmpty.signal();
                } catch (Exception ignored) {
                    return;
                } finally {
                    buffer.lock.unlock();
                }
            }
        }
    }

    static final class Consumer implements Runnable {
        private final Buffer buffer;

        Consumer(Buffer buffer) {
            this.buffer = buffer;
        }

        @Override
        public void run() {
            for (int i = 1; i <= 5; i++) {
                buffer.lock.lock();
                try {
                    while (!buffer.full) {
                        buffer.notEmpty.await();
                    }
                    buffer.consumed = buffer.consumed * 10 + buffer.slot;
                    buffer.full = false;
                    buffer.notFull.signalAll();
                } catch (Exception ignored) {
                    return;
                } finally {
                    buffer.lock.unlock();
                }
            }
        }
    }

    public static int producerConsumerSequence() throws Exception {
        Buffer buffer = new Buffer();
        Thread consumer = new Thread(new Consumer(buffer));
        Thread producer = new Thread(new Producer(buffer));
        consumer.start();
        producer.start();
        producer.join();
        consumer.join();
        return buffer.consumed;
    }

    public static int awaitReacquiresLockAfterSignal() throws Exception {
        ReentrantLock lock = new ReentrantLock();
        Condition condition = lock.newCondition();
        AtomicInteger ready = new AtomicInteger(0);
        AtomicInteger result = new AtomicInteger(0);

        Thread waiter = new Thread(new Runnable() {
            @Override
            public void run() {
                lock.lock();
                try {
                    ready.set(1);
                    condition.await();
                    result.set(lock.isHeldByCurrentThread() ? 1 : -1);
                } catch (Exception ignored) {
                    result.set(-2);
                } finally {
                    lock.unlock();
                }
            }
        });

        waiter.start();
        while (ready.get() == 0) {
            Thread.sleep(1L);
        }
        lock.lock();
        try {
            condition.signal();
        } finally {
            lock.unlock();
        }
        waiter.join();
        return result.get();
    }

    public static int conditionCallsRequireLockOwnership() {
        ReentrantLock lock = new ReentrantLock();
        Condition condition = lock.newCondition();
        int score = 0;
        try {
            condition.signal();
            return -1;
        } catch (IllegalMonitorStateException expected) {
            score += 1;
        }
        try {
            condition.signalAll();
            return -2;
        } catch (IllegalMonitorStateException expected) {
            score += 10;
        }
        try {
            condition.awaitNanos(1L);
            return -3;
        } catch (IllegalMonitorStateException expected) {
            score += 100;
        } catch (Exception unexpected) {
            return -4;
        }
        try {
            condition.await();
            return -5;
        } catch (IllegalMonitorStateException expected) {
            score += 1000;
        } catch (Exception unexpected) {
            return -6;
        }
        return score;
    }
}
