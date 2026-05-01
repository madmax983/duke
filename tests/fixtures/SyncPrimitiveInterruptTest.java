import java.util.concurrent.CountDownLatch;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.Semaphore;
import java.util.concurrent.BrokenBarrierException;
import java.util.concurrent.atomic.AtomicInteger;

public final class SyncPrimitiveInterruptTest {
    public static int latchAwaitInterruptRaises() throws Exception {
        CountDownLatch latch = new CountDownLatch(1);
        AtomicInteger result = new AtomicInteger(0);
        Thread waiter = new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    latch.await();
                    result.set(-1);
                } catch (InterruptedException expected) {
                    result.set(1);
                } catch (Exception unexpected) {
                    result.set(-2);
                }
            }
        });
        waiter.start();
        Thread.sleep(10L);
        waiter.interrupt();
        waiter.join();
        return result.get();
    }

    public static int semaphoreUninterruptiblePreservesFlag() throws Exception {
        Semaphore semaphore = new Semaphore(0);
        AtomicInteger result = new AtomicInteger(0);
        Thread waiter = new Thread(new Runnable() {
            @Override
            public void run() {
                semaphore.acquireUninterruptibly();
                result.set(Thread.interrupted() ? 1 : -1);
            }
        });
        waiter.start();
        Thread.sleep(10L);
        waiter.interrupt();
        semaphore.release();
        waiter.join();
        return result.get();
    }

    public static int barrierInterruptBreaksGeneration() throws Exception {
        CyclicBarrier barrier = new CyclicBarrier(2);
        AtomicInteger first = new AtomicInteger(0);
        AtomicInteger second = new AtomicInteger(0);
        Thread interrupted = new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    barrier.await();
                    first.set(-1);
                } catch (InterruptedException expected) {
                    first.set(1);
                } catch (BrokenBarrierException broken) {
                    first.set(-2);
                } catch (Exception unexpected) {
                    first.set(-3);
                }
            }
        });
        Thread broken = new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    barrier.await();
                    second.set(-1);
                } catch (BrokenBarrierException expected) {
                    second.set(10);
                } catch (Exception unexpected) {
                    second.set(-2);
                }
            }
        });

        interrupted.start();
        Thread.sleep(10L);
        interrupted.interrupt();
        interrupted.join();
        broken.start();
        broken.join();
        return first.get() * 100 + second.get();
    }
}
