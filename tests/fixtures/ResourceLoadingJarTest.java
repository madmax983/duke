import java.io.InputStream;
import java.net.URL;
import java.nio.charset.StandardCharsets;

public final class ResourceLoadingJarTest {
    private static final String EXPECTED = "hello from jar resource\n";

    private ResourceLoadingJarTest() {}

    private static String readAll(InputStream in) throws Exception {
        byte[] chunk = new byte[8];
        byte[] out = new byte[64];
        int size = 0;
        while (true) {
            int read = in.read(chunk, 0, chunk.length);
            if (read < 0) {
                break;
            }
            for (int i = 0; i < read; i++) {
                out[size++] = chunk[i];
            }
        }
        return new String(out, 0, size, StandardCharsets.UTF_8);
    }

    public static int readMetaInfMessage() throws Exception {
        try (InputStream in = ResourceLoadingJarTest.class.getResourceAsStream("/META-INF/messages.txt")) {
            if (in == null) return 1;
            return EXPECTED.equals(readAll(in)) ? 0 : 2;
        }
    }

    public static int urlUsesJarSchemeAndOpens() throws Exception {
        URL url = ResourceLoadingJarTest.class.getResource("/META-INF/messages.txt");
        if (url == null) return 1;
        if (!url.toString().startsWith("jar:file:")) return 2;
        if (!url.toString().equals(url.toExternalForm())) return 3;
        try (InputStream in = url.openStream()) {
            return EXPECTED.equals(readAll(in)) ? 0 : 4;
        }
    }

    public static int runAll() throws Exception {
        if (readMetaInfMessage() != 0) return 101;
        if (urlUsesJarSchemeAndOpens() != 0) return 102;
        return 1;
    }
}
