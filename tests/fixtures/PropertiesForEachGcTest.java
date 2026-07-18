import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Properties;
import java.util.Set;

/**
 * Regression probe for {@code Properties.forEach} ref-safety across a major GC
 * that fires MID-iteration. The consumer RETAINS large arrays in an outer list
 * so old-gen live grows past the 2x major-GC threshold and the collector
 * relocates surviving objects while the native still holds the consumer + the
 * not-yet-visited key/value pairs. If the forEach native fails to re-resolve
 * those held refs after each callback, a later iteration passes a stale
 * (dangling) consumer or key/value and the recorded set will not match the
 * originals. Returns 0 iff all 12 {@code keyN=valN} entries survive intact.
 */
public class PropertiesForEachGcTest {
    // Retains allocations so they survive into old-gen and force major GCs.
    private static final List<int[]> KEEP = new ArrayList<>();

    public static int run() {
        KEEP.clear();
        Properties props = new Properties();
        for (int i = 0; i < 12; i++) {
            props.setProperty("key" + i, "val" + i);
        }

        List<String> seen = new ArrayList<>();
        props.forEach((k, v) -> {
            // Retain sizeable arrays every iteration to grow old-gen live set and
            // trip a major collection partway through the forEach.
            for (int j = 0; j < 40; j++) {
                KEEP.add(new int[512]);
            }
            seen.add(((String) k) + "=" + ((String) v));
        });

        if (seen.size() != 12) return 1;
        Set<String> expected = new HashSet<>();
        for (int i = 0; i < 12; i++) {
            expected.add("key" + i + "=val" + i);
        }
        for (String entry : seen) {
            if (!expected.contains(entry)) return 2;
        }
        return 0;
    }
}
