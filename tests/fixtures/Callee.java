/**
 * Phase 10 fixture — a simple utility class called from CrossCall.
 * All methods are static for invokestatic testing.
 */
public class Callee {
    public static int add(int a, int b) {
        return a + b;
    }

    public static int doubleVal(int n) {
        return n * 2;
    }

    public static int negate(int n) {
        return -n;
    }
}
