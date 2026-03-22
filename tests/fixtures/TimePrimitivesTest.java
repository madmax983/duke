public final class TimePrimitivesTest {
    private TimePrimitivesTest() {
    }

    public static long currentTimeMillisNow() {
        return System.currentTimeMillis();
    }

    public static int currentTimeMillisAdvancesAfterSleep() throws Exception {
        long before = System.currentTimeMillis();
        Thread.sleep(25L);
        long after = System.currentTimeMillis();
        return after > before ? 1 : 0;
    }

    public static long nanoTimeDeltaAfterSleep() throws Exception {
        long before = System.nanoTime();
        Thread.sleep(10L);
        return System.nanoTime() - before;
    }

    public static int nanoTimeSupportsDurationMath() throws Exception {
        long start = System.nanoTime();
        Thread.sleep(10L);
        long elapsedNanos = System.nanoTime() - start;
        long elapsedMillis = elapsedNanos / 1_000_000L;
        return elapsedNanos > 0L && elapsedMillis >= 1L ? 1 : 0;
    }
}
