public class TryWithResources {
    // Static counter — accessible from inner class Res.close() via
    // `putstatic TryWithResources.closeCount:I`.
    static int closeCount = 0;

    // Static nested class — compiles to TryWithResources$Res.class.
    // Implements AutoCloseable so the compiler accepts try-with-resources.
    static class Res implements AutoCloseable {
        @Override
        public void close() { closeCount++; }
    }

    /**
     * Happy path: try body completes normally.
     * Compiler emits: body → close() → return.
     * Returns 42.
     */
    static int simpleValue() {
        try (Res r = new Res()) {
            return 42;
        }
    }

    /**
     * Verify close() is called on normal exit.
     * Returns closeCount after the try block (should be 1).
     */
    static int closedOnSuccess() {
        closeCount = 0;
        try (Res r = new Res()) {
            int x = 0; // non-empty body
        }
        return closeCount;
    }

    /**
     * Exception path: body throws, close() still called, caller catches.
     * Returns closeCount inside the catch (should be 1).
     */
    static int closedOnException() {
        closeCount = 0;
        try (Res r = new Res()) {
            throw new RuntimeException("boom");
        } catch (RuntimeException e) {
            return closeCount; // 1
        }
    }

    /**
     * Nested resources: both r1 and r2 are closed (r2 first, r1 second).
     * Returns closeCount (should be 2).
     */
    static int nestedClosed() {
        closeCount = 0;
        try (Res r1 = new Res(); Res r2 = new Res()) {
            int x = 0; // non-empty body
        }
        return closeCount; // 2
    }
}
