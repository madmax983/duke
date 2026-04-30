import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.util.Collection;
import java.util.Enumeration;
import java.util.Map;
import java.util.Properties;
import java.util.Set;

public class PropertiesBasicTest {
    private static boolean eq(String actual, String expected) {
        if (actual == null) {
            return expected == null;
        }
        return actual.equals(expected);
    }

    public static int loadSample(String path) throws Exception {
        Properties props = new Properties();
        try (FileInputStream in = new FileInputStream(path)) {
            props.load(in);
        }

        if (!eq(props.getProperty("simple"), "value")) return 1;
        if (!eq(props.getProperty("colon"), "colon-value")) return 2;
        if (!eq(props.getProperty("whitespace"), "whitespace-value")) return 3;
        if (!eq(props.getProperty("leading.whitespace"), "trimmed")) return 4;
        if (!eq(props.getProperty("continued"), "firstsecondthird")) return 5;
        if (!eq(props.getProperty("escaped key"), "line\nwith\ttab\\slash")) return 6;
        if (!eq(props.getProperty("unicode"), "Ol\u00E9")) return 7;
        if (!eq(props.getProperty("blank"), "")) return 8;
        return props.getProperty("missing") == null ? 0 : 9;
    }

    public static int getPropertyDefaults() {
        Properties props = new Properties();
        if (props.getProperty("missing") != null) return 1;
        if (!eq(props.getProperty("missing", "fallback"), "fallback")) return 2;
        props.setProperty("present", "actual");
        if (!eq(props.getProperty("present", "fallback"), "actual")) return 3;
        return 0;
    }

    public static int chainedDefaults() {
        Properties defaults = new Properties();
        defaults.setProperty("base", "from-defaults");
        defaults.setProperty("override", "defaults-value");

        Properties props = new Properties(defaults);
        props.setProperty("override", "local-value");

        if (!eq(props.getProperty("base"), "from-defaults")) return 1;
        if (!eq(props.getProperty("override"), "local-value")) return 2;
        if (!props.stringPropertyNames().contains("base")) return 3;
        if (!props.stringPropertyNames().contains("override")) return 4;
        return props.stringPropertyNames().size() == 2 ? 0 : 5;
    }

    public static int setPropertyReturnsPriorValue() {
        Properties props = new Properties();
        Object first = props.setProperty("k", "one");
        if (first != null) return 1;
        Object second = props.setProperty("k", "two");
        if (!"one".equals(second)) return 2;
        return eq(props.getProperty("k"), "two") ? 0 : 3;
    }

    public static int storeRoundTrip(String outPath) throws Exception {
        Properties props = new Properties();
        props.setProperty("alpha", "one");
        props.setProperty("beta", "two\nline");

        try (FileOutputStream out = new FileOutputStream(outPath)) {
            props.store(out, "header");
        }

        Properties loaded = new Properties();
        try (FileInputStream in = new FileInputStream(outPath)) {
            loaded.load(in);
        }

        if (!eq(loaded.getProperty("alpha"), "one")) return 1;
        if (!eq(loaded.getProperty("beta"), "two\nline")) return 2;
        return loaded.stringPropertyNames().size() == 2 ? 0 : 3;
    }

    public static int storeEscapedRoundTrip(String outPath) throws Exception {
        String key = "a=b:c #";
        String value = "x\\y\nz\t\u00E9";
        Properties props = new Properties();
        props.setProperty(key, value);

        try (FileOutputStream out = new FileOutputStream(outPath)) {
            props.store(out, "header");
        }

        Properties loaded = new Properties();
        try (FileInputStream in = new FileInputStream(outPath)) {
            loaded.load(in);
        }

        return eq(loaded.getProperty(key), value) ? 0 : 1;
    }

    public static int propertyNamesIncludeDefaults() {
        Properties defaults = new Properties();
        defaults.setProperty("base", "from-defaults");
        defaults.setProperty("override", "defaults-value");

        Properties props = new Properties(defaults);
        props.setProperty("override", "local-value");
        props.setProperty("local", "local-value");

        Set<String> names = props.stringPropertyNames();
        if (names.size() != 3) return 1;
        if (!names.contains("base")) return 2;
        if (!names.contains("override")) return 3;
        if (!names.contains("local")) return 4;

        Enumeration<?> enumeration = props.propertyNames();
        int seen = 0;
        int count = 0;
        while (enumeration.hasMoreElements()) {
            Object next = enumeration.nextElement();
            if ("base".equals(next)) seen |= 1;
            if ("override".equals(next)) seen |= 2;
            if ("local".equals(next)) seen |= 4;
            count++;
        }
        if (count != 3) return 5;
        return seen == 7 ? 0 : 6;
    }

    public static int localViewsAndMutation() {
        Properties defaults = new Properties();
        defaults.setProperty("base", "from-defaults");

        Properties props = new Properties(defaults);
        props.setProperty("local", "local-value");

        Set<Object> keys = props.keySet();
        Collection<Object> values = props.values();
        Set<Map.Entry<Object, Object>> entries = props.entrySet();

        if (props.size() != 1) return 1;
        if (!props.containsKey("local")) return 2;
        if (props.containsKey("base")) return 3;
        if (keys.size() != 1 || !keys.contains("local") || keys.contains("base")) return 4;
        if (values.size() != 1 || !values.contains("local-value")) return 5;
        if (entries.size() != 1) return 6;
        if (!props.toString().contains("local=local-value")) return 7;

        Object removed = props.remove("local");
        if (!"local-value".equals(removed)) return 8;
        if (!props.isEmpty()) return 9;

        props.setProperty("again", "value");
        props.clear();
        return props.isEmpty() ? 0 : 10;
    }
}
