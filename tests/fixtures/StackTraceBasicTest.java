public class StackTraceBasicTest {
    static int testBasicFrames() {
        try {
            a();
            return -1;
        } catch (IllegalStateException e) {
            StackTraceElement[] frames = e.getStackTrace();
            if (frames.length < 3) {
                return 10 + frames.length;
            }
            StackTraceElement top = frames[0];
            if (!"c".equals(top.getMethodName())) {
                return 20;
            }
            if (top.getLineNumber() <= 0) {
                return 30;
            }
            if (!"StackTraceBasicTest.java".equals(top.getFileName())) {
                return 40;
            }
            if (!"StackTraceBasicTest".equals(top.getClassName())) {
                return 50;
            }
            String rendered = top.toString();
            return rendered.contains("StackTraceBasicTest.c(StackTraceBasicTest.java:") ? frames.length : 60;
        }
    }

    static int testDefensiveCopyAndSetStackTrace() {
        try {
            c();
            return -1;
        } catch (IllegalStateException e) {
            StackTraceElement[] first = e.getStackTrace();
            if (first.length == 0) {
                return 10;
            }
            StackTraceElement original = first[0];
            first[0] = new StackTraceElement("Mutated", "mutated", "Mutated.java", 7);
            if ("Mutated".equals(e.getStackTrace()[0].getClassName())) {
                return 20;
            }
            StackTraceElement[] replacement = new StackTraceElement[] {
                new StackTraceElement("Manual", "frame", "Manual.java", 123)
            };
            e.setStackTrace(replacement);
            replacement[0] = original;
            StackTraceElement[] after = e.getStackTrace();
            if (after.length != 1) {
                return 30;
            }
            if (!"Manual".equals(after[0].getClassName())) {
                return 40;
            }
            if (!"frame".equals(after[0].getMethodName())) {
                return 50;
            }
            if (after[0].getLineNumber() != 123) {
                return 60;
            }
            return 1;
        }
    }

    static int testLocalizedMessageStillFallsBack() {
        IllegalStateException e = new IllegalStateException("hello");
        return "hello".equals(e.getLocalizedMessage()) ? 1 : 0;
    }

    static void a() {
        b();
    }

    static void b() {
        c();
    }

    static void c() {
        throw new IllegalStateException("boom");
    }
}
