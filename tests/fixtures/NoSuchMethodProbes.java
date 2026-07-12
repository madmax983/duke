// Fixtures for the honest-dispatch lane: a call to a method on an unresolvable
// class raises a *catchable* java.lang.NoSuchMethodError (JVMS 5.4.3.3), rather
// than the former silent lenient-dispatch soft-fail (which popped args + `this`
// and continued without pushing a return value, corrupting the operand stack).
//
// `MissingMethods` is compiled alongside these probes so javac can resolve the
// references, but its `.class` is intentionally NOT committed (deleted after
// compilation) so that at run time it cannot be loaded. `new MissingMethods()`
// emits `invokespecial MissingMethods.<init>()V` against the unloadable class,
// which the interpreter now reports as a catchable NoSuchMethodError whose
// detail message names the owner class, method, and descriptor.
public class NoSuchMethodProbes {

    // Catchable directly as NoSuchMethodError; the detail message names the
    // owner class, method, and descriptor (probe returns 1 only if all hold).
    public static int probeCatchNoSuchMethodError() {
        try {
            MissingMethods m = new MissingMethods();
            return m.value();
        } catch (NoSuchMethodError e) {
            String msg = e.getMessage();
            return (msg != null
                    && msg.contains("MissingMethods")
                    && msg.contains("<init>")
                    && msg.contains("()V")) ? 1 : 2;
        }
    }

    // Catchable as its immediate superclass IncompatibleClassChangeError.
    public static int probeCatchIncompatibleClassChangeError() {
        try {
            new MissingMethods();
            return 3;
        } catch (IncompatibleClassChangeError e) {
            return 1;
        }
    }

    // Catchable as LinkageError (grandparent in the hierarchy).
    public static int probeCatchLinkageError() {
        try {
            new MissingMethods();
            return 3;
        } catch (LinkageError e) {
            return 1;
        }
    }

    // Catchable as the Throwable root.
    public static int probeCatchThrowable() {
        try {
            new MissingMethods();
            return 3;
        } catch (Throwable e) {
            return 1;
        }
    }

    // No handler: the NoSuchMethodError propagates uncaught so the Rust harness
    // can assert the exact Error::JavaException class name.
    public static int uncaughtNoSuchMethod() {
        MissingMethods m = new MissingMethods();
        return m.value();
    }
}

class MissingMethods {
    MissingMethods() {
    }

    int value() {
        return 42;
    }
}
