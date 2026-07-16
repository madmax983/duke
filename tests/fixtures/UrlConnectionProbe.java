import java.io.InputStream;
import java.net.URL;
import java.net.URLConnection;
import java.nio.charset.StandardCharsets;

/**
 * Exercises the Spring `UrlResource.getInputStream()` shape:
 * `url.openConnection().getInputStream()`, reading a classpath resource
 * end-to-end through Duke's synthetic `java/net/URLConnection`.
 */
public final class UrlConnectionProbe {
    private static final String EXPECTED = "hello duke\nresource line 2\n";

    /** Exposed so the driving test can assert the exact byte count that flowed through. */
    public static int lastLength = -1;

    private UrlConnectionProbe() {}

    private static String readAll(InputStream in) throws Exception {
        byte[] chunk = new byte[5];
        byte[] out = new byte[128];
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
        lastLength = size;
        return new String(out, 0, size, StandardCharsets.UTF_8);
    }

    /**
     * URL -> openConnection() -> setUseCaches(false) -> getInputStream() -> read all bytes.
     * Returns 0 on an exact-content match, otherwise a distinct non-zero diagnostic code.
     */
    public static int openConnectionReadsResource() throws Exception {
        URL url = UrlConnectionProbe.class.getResource("/ResourceLoadingTestData.txt");
        if (url == null) return 1;
        URLConnection connection = url.openConnection();
        if (connection == null) return 2;
        connection.setUseCaches(false);
        try (InputStream in = connection.getInputStream()) {
            if (in == null) return 3;
            String content = readAll(in);
            if (lastLength != EXPECTED.length()) return 4;
            return EXPECTED.equals(content) ? 0 : 5;
        }
    }
}
