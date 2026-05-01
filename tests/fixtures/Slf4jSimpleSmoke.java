import org.slf4j.LoggerFactory;
import java.net.URL;
import java.net.URLClassLoader;

public final class Slf4jSimpleSmoke {
    private Slf4jSimpleSmoke() {}

    public static int simpleLoggerPropertiesStreamIsNull() throws Exception {
        URL classUrl = org.slf4j.simple.SimpleLogger.class.getResource("/org/slf4j/simple/SimpleLogger.class");
        if (classUrl == null) {
            return 2;
        }
        String spec = classUrl.toString();
        int bang = spec.indexOf("!/");
        if (bang < 0 || !spec.startsWith("jar:")) {
            return 3;
        }
        String rootSpec = spec.substring(4, bang);
        URLClassLoader loader = new URLClassLoader(new URL[] { new URL(rootSpec) });
        return loader.getResourceAsStream("simplelogger.properties") == null ? 1 : 0;
    }

    public static void main(String[] args) {
        LoggerFactory.getLogger("duke-smoke").info("hello {}", "world");
    }
}
