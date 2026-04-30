import java.util.concurrent.atomic.AtomicBoolean;

public final class AtomicBooleanBasicTest {
    public static int basicOperations() {
        AtomicBoolean value = new AtomicBoolean(false);
        if (value.get()) return -1;
        if (!value.compareAndSet(false, true)) return -2;
        if (!value.get()) return -3;
        if (value.compareAndSet(false, true)) return -4;
        if (!value.getAndSet(false)) return -5;
        if (value.get()) return -6;
        if (!"false".equals(value.toString())) return -7;
        return 1;
    }
}
