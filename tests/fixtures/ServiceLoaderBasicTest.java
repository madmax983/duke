import com.example.Greeter;
import java.util.Iterator;
import java.util.ServiceLoader;

public final class ServiceLoaderBasicTest {
    public static int providersInDeclarationOrder() {
        ServiceLoader<Greeter> loader = ServiceLoader.load(Greeter.class);
        Iterator<Greeter> first = loader.iterator();
        if (!first.hasNext()) {
            return -1;
        }
        Greeter hello = first.next();
        if (!"hello".equals(hello.greet())) {
            return -2;
        }
        if (!first.hasNext()) {
            return -3;
        }
        Greeter howdy = first.next();
        if (!"howdy".equals(howdy.greet())) {
            return -4;
        }
        if (first.hasNext()) {
            return -5;
        }

        Iterator<Greeter> second = loader.iterator();
        return second.hasNext() && "hello".equals(second.next().greet()) ? 1 : -6;
    }
}
