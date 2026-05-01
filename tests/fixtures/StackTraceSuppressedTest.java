public class StackTraceSuppressedTest {
    static class FailingClose implements AutoCloseable {
        @Override
        public void close() {
            throw new IllegalArgumentException("close");
        }
    }

    static int testSuppressedIsRetained() {
        try (FailingClose ignored = new FailingClose()) {
            throw new IllegalStateException("body");
        } catch (IllegalStateException e) {
            Throwable[] suppressed = e.getSuppressed();
            if (suppressed.length != 1) {
                return 10 + suppressed.length;
            }
            if (!(suppressed[0] instanceof IllegalArgumentException)) {
                return 20;
            }
            return "close".equals(suppressed[0].getMessage()) ? 1 : 30;
        }
    }
}
