import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * Companion to {@link PropertiesForEachGcTest}: drives the audited
 * {@code native_hashmap_for_each} through the SAME mid-iteration major-GC
 * pressure. It fails identically, proving the ref-safety gap is family-wide
 * (GC-core), not specific to any one synthetic forEach native.
 *
 * <p>The consumer RETAINS large arrays so old-gen live grows past the 2x major-GC
 * threshold and the collector relocates surviving objects (repeatedly) while the
 * native still holds the consumer + not-yet-visited key/value pairs as Rust
 * locals. Those locals are not GC roots and are not patched by the interpreter's
 * post-collection sweep, and a single {@code apply_forward} cannot recover a ref
 * that moved across more than one collection. Returns 0 iff all 12 entries
 * survive intact.
 */
public class HashMapForEachGcTest {
    private static final List<int[]> KEEP = new ArrayList<>();

    public static int run() {
        KEEP.clear();
        Map<String, String> m = new HashMap<>();
        for (int i = 0; i < 12; i++) {
            m.put("key" + i, "val" + i);
        }

        List<String> seen = new ArrayList<>();
        m.forEach((k, v) -> {
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
