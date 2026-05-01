import org.slf4j.LoggerFactory;

public final class Slf4jSimpleSmoke {
    private Slf4jSimpleSmoke() {}

    public static void main(String[] args) {
        LoggerFactory.getLogger("duke-smoke").info("hello {}", "world");
    }
}
