public class MonitorAndAbstract {
    public static int syncBlock(int x) {
        Object lock = new Object();
        int result;
        synchronized (lock) {
            result = x * 2;
        }
        return result;
    }

    public static synchronized int syncMethod(int x) {
        return x + 10;
    }

    public static int nestedSync(int x) {
        Object a = new Object();
        Object b = new Object();
        int r;
        synchronized (a) {
            synchronized (b) {
                r = x + 5;
            }
        }
        return r;
    }
}
