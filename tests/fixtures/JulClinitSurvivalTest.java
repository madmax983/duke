import java.util.concurrent.atomic.AtomicLong;
import java.util.logging.Logger;
import java.util.regex.Pattern;

public final class JulClinitSurvivalTest {
    private static final Logger LOG = Logger.getLogger(JulClinitSurvivalTest.class.getName());
    private static final AtomicLong COUNTER = new AtomicLong(0);
    private static final Pattern P = Pattern.compile("^x.*$");

    public static void main(String[] args) {
        System.out.println(LOG != null && COUNTER != null && P != null);
    }

    public static int clinitSurvives() {
        return LOG != null && COUNTER != null && P != null ? 1 : -1;
    }
}
