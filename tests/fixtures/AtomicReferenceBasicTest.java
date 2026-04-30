import java.util.concurrent.atomic.AtomicReference;

public final class AtomicReferenceBasicTest {
    public static int basicOperations() {
        AtomicReference<String> value = new AtomicReference<>();
        if (value.get() != null) return -1;

        value.set("hello");
        if (!"hello".equals(value.get())) return -2;
        if (!"hello".equals(value.getAndSet("world"))) return -3;
        if (!"world".equals(value.get())) return -4;

        String r1 = new StringBuilder("world").toString();
        String r2 = new StringBuilder("world").toString();
        String r3 = new StringBuilder("world").toString();
        value.set(r1);
        if (!value.compareAndSet(r1, r2)) return -5;
        if (value.compareAndSet(r1, r3)) return -6;
        if (value.get() != r2) return -7;
        if (!"world".equals(value.toString())) return -8;
        return 1;
    }
}
