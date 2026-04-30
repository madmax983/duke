import java.util.logging.Level;
import java.util.logging.Logger;

public final class JulLoggerBasicTest {
    private static final Logger LOG = Logger.getLogger(JulLoggerBasicTest.class.getName());

    public static void main(String[] args) {
        LOG.info("hello");
        LOG.warning("warn");
        LOG.fine("dropped");
        System.out.println("JulLoggerBasicTest".equals(LOG.getName())
                && LOG.isLoggable(Level.INFO)
                && !LOG.isLoggable(Level.FINE));
    }

    public static int run() {
        LOG.info("hello");
        LOG.warning("warn");
        LOG.fine("dropped");
        if (!"JulLoggerBasicTest".equals(LOG.getName())) return -1;
        if (!LOG.isLoggable(Level.INFO)) return -2;
        if (LOG.isLoggable(Level.FINE)) return -3;
        return 1;
    }
}
