import java.util.zip.ZipFile;
import java.util.zip.ZipEntry;
import java.io.InputStream;

public class ZipReadTest {
    public static int entryCount(String path) throws Exception {
        ZipFile zf = new ZipFile(path);
        int count = zf.size();
        zf.close();
        return count;
    }

    public static int readFirstByte(String path, String entryName) throws Exception {
        ZipFile zf = new ZipFile(path);
        ZipEntry entry = zf.getEntry(entryName);
        InputStream is = zf.getInputStream(entry);
        int b = is.read();
        is.close();
        zf.close();
        return b;
    }
}
