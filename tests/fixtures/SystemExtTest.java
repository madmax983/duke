public class SystemExtTest {
    static int testErr() {
        System.err.println("error output");
        return 1;  // just verifies it doesn't crash
    }

    static int testLineSeparator() {
        String sep = System.lineSeparator();
        return sep.length();  // 1 (\n)
    }

    static int testIdentityHashCode() {
        Object a = new Object();
        int h = System.identityHashCode(a);
        return h >= 0 ? 1 : 0;  // non-negative
    }

    static int testIdentityHashCodeNull() {
        return System.identityHashCode(null);  // 0
    }

    @SuppressWarnings("removal")
    static int testGetSecurityManager() {
        return System.getSecurityManager() == null ? 1 : 0;
    }
}
