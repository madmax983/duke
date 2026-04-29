import com.example.Greeter;
import java.util.Iterator;
import java.util.ServiceConfigurationError;
import java.util.ServiceLoader;

public final class ServiceLoaderMalformedTest {
    public static int missingProviderThrowsButIteratorContinues() {
        Iterator<Greeter> iterator = ServiceLoader.load(Greeter.class).iterator();
        try {
            iterator.next();
            return -1;
        } catch (ServiceConfigurationError expected) {
            String message = expected.getMessage();
            if (message == null || !message.contains("com.example.MissingGreeter")) {
                return -2;
            }
        }

        if (!iterator.hasNext()) {
            return -3;
        }
        Greeter provider = iterator.next();
        if (!"hello".equals(provider.greet())) {
            return -4;
        }
        return iterator.hasNext() ? -5 : 1;
    }
}
