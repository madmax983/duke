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
}
