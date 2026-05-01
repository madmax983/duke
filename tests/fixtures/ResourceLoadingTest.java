import java.io.IOException;
import java.io.InputStream;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;

public final class ResourceLoadingTest {
    private static final String EXPECTED = "hello duke\nresource line 2\n";

    private ResourceLoadingTest() {}

    private static String readAll(InputStream in) throws Exception {
        byte[] chunk = new byte[5];
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

    public static int absoluteFromClassReturnsBytes() throws Exception {
        try (InputStream in = ResourceLoadingTest.class.getResourceAsStream("/ResourceLoadingTestData.txt")) {
            if (in == null) return 1;
            return EXPECTED.equals(readAll(in)) ? 0 : 2;
        }
    }

    public static int relativeFromClassReturnsBytes() throws Exception {
        try (InputStream in = ResourceLoadingTest.class.getResourceAsStream("ResourceLoadingTestData.txt")) {
            if (in == null) return 1;
            return EXPECTED.equals(readAll(in)) ? 0 : 2;
        }
    }

    private static URLClassLoader openFixtureClassLoader() throws Exception {
        URL resourceUrl = ResourceLoadingTest.class.getResource("/ResourceLoadingTestData.txt");
        if (resourceUrl == null) {
            return null;
        }
        String rootSpec = resourceUrl.toString().replace("ResourceLoadingTestData.txt", "");
        return new URLClassLoader(new URL[] { new URL(rootSpec) });
    }

    public static int classLoaderResourceAsStreamReturnsBytes() throws Exception {
        URLClassLoader loader = openFixtureClassLoader();
        if (loader == null) return 1;
        try (InputStream in = loader.getResourceAsStream("ResourceLoadingTestData.txt")) {
            if (in == null) return 1;
            return EXPECTED.equals(readAll(in)) ? 0 : 2;
        }
    }

    public static int missingResourceReturnsNull() {
        return ResourceLoadingTest.class.getResourceAsStream("does-not-exist.txt") == null ? 0 : 1;
    }

    public static int traversalAttemptReturnsNull() {
        return ResourceLoadingTest.class.getResourceAsStream("/../../../etc/passwd") == null ? 0 : 1;
    }

    public static int closedStreamThrowsIOException() throws Exception {
        InputStream in = ResourceLoadingTest.class.getResourceAsStream("/ResourceLoadingTestData.txt");
        if (in == null) return 1;
        in.close();
        try {
            in.read();
            return 2;
        } catch (IOException expected) {
            return "Stream closed".equals(expected.getMessage()) ? 0 : 3;
        }
    }

    public static int availableSkipAndSliceReadWork() throws Exception {
        try (InputStream in = ResourceLoadingTest.class.getResourceAsStream("/ResourceLoadingTestData.txt")) {
            if (in == null) return 1;
            if (in.available() != EXPECTED.length()) return 2;
            long skipped = in.skip(6);
            if (skipped != 6L) return 3;
            if (in.available() != EXPECTED.length() - 6) return 4;
            byte[] target = new byte[12];
            int read = in.read(target, 2, 4);
            if (read != 4) return 5;
            String chunk = new String(target, 2, 4, StandardCharsets.UTF_8);
            return "duke".equals(chunk) ? 0 : 6;
        }
    }

    public static int urlOpenStreamReadsBytes() throws Exception {
        URL url = ResourceLoadingTest.class.getResource("/ResourceLoadingTestData.txt");
        if (url == null) return 1;
        if (!url.toString().startsWith("file:")) return 2;
        if (!url.toString().equals(url.toExternalForm())) return 3;
        if (url.getPath() == null || url.getPath().isEmpty()) return 4;
        try (InputStream in = url.openStream()) {
            return EXPECTED.equals(readAll(in)) ? 0 : 5;
        }
    }

    public static int classLoaderGetResourceReturnsUrl() throws Exception {
        URLClassLoader loader = openFixtureClassLoader();
        if (loader == null) return 1;
        URL url = loader.getResource("ResourceLoadingTestData.txt");
        if (url == null) return 1;
        if (!url.toString().startsWith("file:")) return 2;
        try (InputStream in = url.openStream()) {
            return EXPECTED.equals(readAll(in)) ? 0 : 3;
        }
    }

    public static int runAll() throws Exception {
        if (absoluteFromClassReturnsBytes() != 0) return 101;
        if (relativeFromClassReturnsBytes() != 0) return 102;
        if (classLoaderResourceAsStreamReturnsBytes() != 0) return 103;
        if (missingResourceReturnsNull() != 0) return 104;
        if (traversalAttemptReturnsNull() != 0) return 105;
        if (closedStreamThrowsIOException() != 0) return 106;
        if (availableSkipAndSliceReadWork() != 0) return 107;
        if (urlOpenStreamReadsBytes() != 0) return 108;
        if (classLoaderGetResourceReturnsUrl() != 0) return 109;
        return 1;
    }
}
