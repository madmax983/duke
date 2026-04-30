import java.util.concurrent.atomic.AtomicLong;

public final class AtomicLongBasicTest {
    public static int basicOperations() {
        long wide = 5_000_000_000L;
        AtomicLong value = new AtomicLong(wide);
        if (value.get() != wide) return -1;

        value.set(wide + 1L);
        if (value.get() != wide + 1L) return -2;
        if (value.getAndIncrement() != wide + 1L) return -3;
        if (value.incrementAndGet() != wide + 3L) return -4;
        if (value.getAndAdd(5L) != wide + 3L) return -5;
        if (value.get() != wide + 8L) return -6;
        if (!value.compareAndSet(wide + 8L, wide + 100L)) return -7;
        if (value.get() != wide + 100L) return -8;
        if (value.compareAndSet(wide + 8L, wide + 200L)) return -9;
        if (value.get() != wide + 100L) return -10;

        value.lazySet(Long.MAX_VALUE);
        if (value.incrementAndGet() != Long.MIN_VALUE) return -11;
        if (value.getAndSet(42L) != Long.MIN_VALUE) return -12;
        if (value.longValue() != 42L) return -13;
        if (value.intValue() != 42) return -14;
        if (value.floatValue() != 42.0f) return -15;
        if (value.doubleValue() != 42.0d) return -16;
        if (!"42".equals(value.toString())) return -17;
        return 1;
    }
}
