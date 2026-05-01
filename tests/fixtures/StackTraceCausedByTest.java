import java.io.ByteArrayOutputStream;
import java.io.PrintStream;

public class StackTraceCausedByTest {
    static int testPrintStackTraceCause() {
        try {
            wrap();
            return -1;
        } catch (RuntimeException e) {
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            PrintStream ps = new PrintStream(bytes);
            e.printStackTrace(ps);
            String trace = bytes.toString();
            if (!trace.contains("java.lang.IllegalStateException: outer")) {
                return 10;
            }
            if (!trace.contains("Caused by:")) {
                return 20;
            }
            if (!trace.contains("java.lang.IllegalArgumentException: inner")) {
                return 30;
            }
            if (!trace.contains("\tat StackTraceCausedByTest.inner(")) {
                return 40;
            }
            return trace.contains("\tat ") ? 1 : 50;
        }
    }

    static void wrap() {
        try {
            inner();
        } catch (IllegalArgumentException e) {
            throw new IllegalStateException("outer", e);
        }
    }

    static void inner() {
        throw new IllegalArgumentException("inner");
    }
}
