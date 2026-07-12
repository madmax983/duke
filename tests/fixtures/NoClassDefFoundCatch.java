// Fixture for the interpreter linkage-error lane.
//
// `probe()` invokes a static method on `NoClassDefFoundCatchHelper`. The helper
// is compiled alongside this class so javac can resolve the reference, but its
// `.class` file is intentionally NOT committed (deleted after compilation), so
// at run time the referenced class cannot be loaded. A real JVM raises a
// catchable `java.lang.NoClassDefFoundError` at the failing `invokestatic`; the
// `catch (NoClassDefFoundError e)` here must observe it and return 1.
public class NoClassDefFoundCatch {
    public static int probe() {
        try {
            return NoClassDefFoundCatchHelper.value();
        } catch (NoClassDefFoundError e) {
            return 1;
        }
    }
}

class NoClassDefFoundCatchHelper {
    static int value() {
        return 42;
    }
}
