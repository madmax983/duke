import java.io.ByteArrayOutputStream;
import java.io.PrintStream;

public class StackTraceFormatGoldenTest {
    static int testPrintStackTraceFormat() {
        try {
            top();
            return -1;
        } catch (IllegalStateException e) {
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            e.printStackTrace(new PrintStream(bytes));
            String trace = normalizeLines(bytes.toString());
            String expected =
                "java.lang.IllegalStateException: golden\n" +
                "\tat StackTraceFormatGoldenTest.leaf(StackTraceFormatGoldenTest.java:#)\n" +
                "\tat StackTraceFormatGoldenTest.middle(StackTraceFormatGoldenTest.java:#)\n" +
                "\tat StackTraceFormatGoldenTest.top(StackTraceFormatGoldenTest.java:#)\n" +
                "\tat StackTraceFormatGoldenTest.testPrintStackTraceFormat(StackTraceFormatGoldenTest.java:#)\n";
            return expected.equals(trace) ? 1 : 0;
        }
    }

    static void top() {
        middle();
    }

    static void middle() {
        leaf();
    }

    static void leaf() {
        throw new IllegalStateException("golden");
    }

    static String normalizeLines(String input) {
        StringBuilder out = new StringBuilder();
        boolean inSourceLine = false;
        boolean wroteMarker = false;
        for (int i = 0; i < input.length(); i++) {
            char ch = input.charAt(i);
            if (ch == ':') {
                inSourceLine = true;
                wroteMarker = false;
                out.append(ch);
                continue;
            }
            if (inSourceLine && ch >= '0' && ch <= '9') {
                if (!wroteMarker) {
                    out.append('#');
                    wroteMarker = true;
                }
                continue;
            }
            inSourceLine = false;
            out.append(ch);
        }
        return out.toString();
    }
}
