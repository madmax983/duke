import com.example.Greeter;
import java.util.Iterator;
import java.util.ServiceLoader;

public final class ServiceLoaderCommentsTest {
    public static int ignoresCommentsAndBlankLines() {
        Iterator<Greeter> iterator = ServiceLoader.load(Greeter.class).iterator();
        int count = 0;
        String joined = "";
        while (iterator.hasNext()) {
            joined = joined + iterator.next().greet();
            count++;
        }
        return count == 2 && "hellohowdy".equals(joined) ? 1 : -count;
    }
}
