import java.util.concurrent.atomic.AtomicInteger;

public final class AtomicIntegerBasicTest {
    public static int basicOperations() {
        AtomicInteger value = new AtomicInteger();
        if (value.get() != 0) return -1;

        value.set(0);
        if (value.incrementAndGet() != 1) return -2;
        if (value.incrementAndGet() != 2) return -3;
        if (value.incrementAndGet() != 3) return -4;
        if (value.getAndAdd(5) != 3) return -5;
        if (value.get() != 8) return -6;
        if (!value.compareAndSet(8, 100)) return -7;
        if (value.get() != 100) return -8;
        if (value.compareAndSet(8, 200)) return -9;
        if (value.get() != 100) return -10;

        value.lazySet(Integer.MAX_VALUE);
        if (value.incrementAndGet() != Integer.MIN_VALUE) return -11;
        if (value.getAndSet(42) != Integer.MIN_VALUE) return -12;
        if (value.intValue() != 42) return -13;
        if (value.longValue() != 42L) return -14;
        if (value.floatValue() != 42.0f) return -15;
        if (value.doubleValue() != 42.0d) return -16;
        if (!"42".equals(value.toString())) return -17;
        return 1;
    }
}
