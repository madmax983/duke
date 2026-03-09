public class ExceptionTest {

    /**
     * Basic try/catch: throw RuntimeException, catch it, return 42.
     */
    public static int catchSimple() {
        try {
            throw new RuntimeException();
        } catch (RuntimeException e) {
            return 42;
        }
    }

    /**
     * Uncaught throw: no handler -> propagates as VmError::JavaException.
     */
    public static int uncaught() {
        throw new RuntimeException();
    }

    /**
     * Catch-all (finally): try { result=10; } finally { result+=1; } return result;
     */
    public static int finallyBlock() {
        int result = 0;
        try {
            result = 10;
        } finally {
            result += 1;
        }
        return result;
    }

    /**
     * Dense switch -> tableswitch instruction.
     */
    public static int switchDense(int x) {
        switch (x) {
            case 0: return 10;
            case 1: return 20;
            case 2: return 30;
            default: return -1;
        }
    }

    /**
     * Sparse switch -> lookupswitch instruction.
     */
    public static int switchSparse(int x) {
        switch (x) {
            case 100: return 1;
            case 200: return 2;
            case 300: return 3;
            default: return 0;
        }
    }

    /**
     * Caller catches exception thrown by callee.
     * Tests cross-frame exception unwinding.
     */
    public static int catchFromCallee() {
        try {
            uncaught(); // throws RuntimeException
            return 0;   // never reached
        } catch (RuntimeException e) {
            return 99;
        }
    }

    /**
     * Throw and catch in same frame — alias for telemetry tests.
     */
    public static int throwAndCatch() {
        try {
            throw new RuntimeException("test");
        } catch (RuntimeException e) {
            return 42;
        }
    }

    /**
     * Rethrow: inner catch rethrows, outer catch handles it.
     */
    public static int rethrow() {
        try {
            try {
                throw new RuntimeException("inner");
            } catch (RuntimeException e) {
                throw e;
            }
        } catch (RuntimeException e) {
            return 99;
        }
    }
}
