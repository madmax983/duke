import java.io.FileDescriptor;
import java.io.FileOutputStream;
import java.io.OutputStreamWriter;

/**
 * Stage-c discovery probe: drives the REAL {@code StreamEncoder}/{@code Charset}
 * writer graph one level above the landed {@code FileOutputStream} floor.
 *
 * Under real-JDK shadow, {@code OutputStreamWriter}/{@code StreamEncoder} are not
 * allowlisted, so their real java.base bytecode loads and runs. This constructs
 * the writer over {@code FileDescriptor.out}, writes a string, and flushes — which
 * forces {@code Charset.forName}/{@code defaultCharset} and the encode path.
 */
public final class StreamEncoderWriterProbe {
    private StreamEncoderWriterProbe() {}

    public static void main(String[] args) throws Exception {
        OutputStreamWriter w =
            new OutputStreamWriter(new FileOutputStream(FileDescriptor.out), "UTF-8");
        w.write("hello\n");
        w.flush();
    }
}
