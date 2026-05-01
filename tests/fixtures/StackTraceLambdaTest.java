public class StackTraceLambdaTest {
    interface ThrowingRunnable {
        void run();
    }

    static int testLambdaTraceIncludesUserCaller() {
        try {
            invokeLambda();
            return -1;
        } catch (IllegalStateException e) {
            StackTraceElement[] frames = e.getStackTrace();
            if (frames.length < 2) {
                return 10 + frames.length;
            }
            if (!"lambda$invokeLambda$0".equals(frames[0].getMethodName())) {
                return 20;
            }
            for (int i = 1; i < frames.length; i++) {
                if ("invokeLambda".equals(frames[i].getMethodName())) {
                    return 1;
                }
            }
            return 30;
        }
    }

    static void invokeLambda() {
        ThrowingRunnable runnable = () -> {
            throw new IllegalStateException("lambda");
        };
        runnable.run();
    }
}
