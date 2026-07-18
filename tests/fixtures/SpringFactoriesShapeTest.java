import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Properties;

/**
 * Reproduces the exact collections shape that Spring Boot's
 * {@code SpringFactoriesLoader.loadFactoriesResource} exercises during startup:
 * a {@link Properties} populated from key/value pairs is iterated with
 * {@code forEach}, and each entry is folded into a {@link LinkedHashMap} via
 * {@code computeIfAbsent(name.trim(), k -> new ArrayList<>(names.length))}.
 *
 * <p>This pins three java.util rungs that Duke previously lacked or mishandled:
 * <ul>
 *   <li>{@code Properties.forEach} must yield the real string KEYS (Duke used to
 *       fall through to the Hashtable/HashMap forEach, which read the wrong slot
 *       — the Properties {@code defaults} field — and handed the consumer a null
 *       key, an NPE on {@code name.trim()});</li>
 *   <li>{@code LinkedHashMap.computeIfAbsent} (and the rest of the Map-default
 *       family) reusing the HashMap natives;</li>
 *   <li>{@code ArrayDeque.<init>(int)} — the initial-capacity constructor.</li>
 * </ul>
 */
public class SpringFactoriesShapeTest {
    public static int run() {
        Properties props = new Properties();
        props.setProperty("com.example.Foo", "a,b,c");
        props.setProperty("com.example.Bar", "d");

        Map<String, List<String>> result = new LinkedHashMap<>();
        props.forEach((name, value) -> {
            String[] names = ((String) value).split(",");
            List<String> list =
                    result.computeIfAbsent(((String) name).trim(), k -> new ArrayList<>(names.length));
            for (String n : names) {
                list.add(n.trim());
            }
        });

        if (result.size() != 2) return 1;
        List<String> foo = result.get("com.example.Foo");
        if (foo == null || foo.size() != 3) return 2;
        if (!foo.get(0).equals("a") || !foo.get(1).equals("b") || !foo.get(2).equals("c")) return 3;
        List<String> bar = result.get("com.example.Bar");
        if (bar == null || bar.size() != 1 || !bar.get(0).equals("d")) return 4;

        // ArrayDeque initial-capacity ctor: java/util/ArrayDeque.<init>(I)V.
        ArrayDeque<String> dq = new ArrayDeque<>(16);
        dq.push("x");
        dq.push("y");
        if (dq.size() != 2) return 5;
        if (!dq.pop().equals("y")) return 6;
        if (!dq.pop().equals("x")) return 7;
        if (!dq.isEmpty()) return 8;

        return 0;
    }
}
