// Fixtures for the interpreter linkage-error lane (catchable NoClassDefFoundError
// + JVMS 5.5 erroneous-class semantics).
//
// `Missing` is compiled alongside these probes so javac can resolve the
// references, but its `.class` is intentionally NOT committed (deleted after
// compilation) so that at run time it cannot be loaded. Each opcode that
// references it must raise a catchable `NoClassDefFoundError`.
//
// `ClinitThrowsError` and `ClinitThrowsRuntime` ARE committed (loadable); their
// `<clinit>` throws to exercise the erroneous-class / ExceptionInInitializerError
// paths.
public class LinkageErrorProbes {

    // invokestatic against a missing class -> catchable NoClassDefFoundError
    // whose detail message names the missing class.
    public static int probeInvokestatic() {
        try {
            return Missing.compute();
        } catch (NoClassDefFoundError e) {
            String m = e.getMessage();
            return (m != null && m.contains("Missing")) ? 1 : 2;
        }
    }

    // new against a missing class -> catchable NoClassDefFoundError.
    public static int probeNew() {
        try {
            Object o = new Missing();
            return o == null ? 3 : 3;
        } catch (NoClassDefFoundError e) {
            return 1;
        }
    }

    // getstatic against a missing class -> catchable NoClassDefFoundError.
    public static int probeGetstatic() {
        try {
            return Missing.VALUE;
        } catch (NoClassDefFoundError e) {
            return 1;
        }
    }

    // putstatic against a missing class -> catchable NoClassDefFoundError.
    public static int probePutstatic() {
        try {
            Missing.VALUE = 5;
            return 3;
        } catch (NoClassDefFoundError e) {
            return 1;
        }
    }

    // commons-logging style: the missing-class linkage error is caught as its
    // superclass LinkageError and the caller falls through to an alternative.
    public static int probeLinkageErrorFallback() {
        int result;
        try {
            Missing.compute();
            result = 3;
        } catch (LinkageError e) {
            result = fallbackValue();
        }
        return result;
    }

    // caught as Throwable (the root of the hierarchy).
    public static int probeThrowableCatch() {
        try {
            Missing.compute();
            return 3;
        } catch (Throwable e) {
            return 1;
        }
    }

    // A class whose <clinit> throws an Error must propagate that Error unwrapped.
    public static int probeClinitErrorUnwrapped() {
        try {
            return ClinitThrowsError.use();
        } catch (ExceptionInInitializerError wrapped) {
            return 2; // must NOT be wrapped
        } catch (Error e) {
            return 1;
        }
    }

    // A class whose <clinit> throws a plain Exception must surface as a
    // catchable ExceptionInInitializerError.
    public static int probeClinitRuntimeWrapped() {
        try {
            return ClinitThrowsRuntime.use();
        } catch (ExceptionInInitializerError e) {
            return 1;
        }
    }

    // After a <clinit> failure the class is erroneous: a second use must raise
    // NoClassDefFoundError instead of re-running the initializer.
    public static int probeErroneousSecondUse() {
        try {
            ClinitThrowsError.use();
        } catch (Throwable ignored) {
            // first use fails and is caught
        }
        try {
            return ClinitThrowsError.use();
        } catch (NoClassDefFoundError e) {
            return 1;
        }
    }

    // No handler: the NoClassDefFoundError propagates uncaught so the Rust
    // harness can assert the exact Error::JavaException class name.
    public static int uncaughtInvokestatic() {
        return Missing.compute();
    }

    static int fallbackValue() {
        return 1;
    }
}

class Missing {
    static int VALUE = 7;

    Missing() {
    }

    static int compute() {
        return 42;
    }
}

class ClinitThrowsError {
    static boolean FLAG = true;

    static {
        if (FLAG) {
            throw new Error("clinit-error-boom");
        }
    }

    static int use() {
        return 9;
    }
}

class ClinitThrowsRuntime {
    static boolean FLAG = true;

    static {
        if (FLAG) {
            throw new RuntimeException("clinit-runtime-boom");
        }
    }

    static int use() {
        return 9;
    }
}
