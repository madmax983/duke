import java.util.logging.Logger;

public final class JulLoggerInterningTest {
    public static void main(String[] args) {
        Logger a = Logger.getLogger("x.y.z");
        Logger b = Logger.getLogger("x.y.z");
        if (a != b) throw new AssertionError();
    }

    public static int sameNameIdentity() {
        Logger a = Logger.getLogger("x.y.z");
        Logger b = Logger.getLogger("x.y.z");
        return a == b ? 1 : -1;
    }
}
