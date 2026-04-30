import com.example.AtomicCounterService;
import java.util.Iterator;
import java.util.ServiceLoader;

public final class AtomicServiceLoaderInteropTest {
    public static int providerClinitUsesAtomicLong() {
        ServiceLoader<AtomicCounterService> loader = ServiceLoader.load(AtomicCounterService.class);
        Iterator<AtomicCounterService> iterator = loader.iterator();
        if (!iterator.hasNext()) return -1;
        AtomicCounterService service = iterator.next();
        if (service.constructedCount() != 1L) return -2;
        if (iterator.hasNext()) return -3;
        return 1;
    }
}
