import java.io.*;

/**
 * Stage-c discovery probe: drives the REAL {@code OutputStreamWriter}/
 * {@code StreamEncoder}/{@code Charset} writer graph WITHOUT touching
 * {@code java.io.FileOutputStream} (which is blocked behind a forbidden
 * {@code KEEP_SYNTHETIC} allowlist edit).
 *
 * The sink is a USER-DEFINED {@code OutputStream} subclass, which loads as REAL
 * bytecode (not synthetic, not allowlisted). Wrapping it in a real
 * {@code OutputStreamWriter} forces the real
 * {@code OutputStreamWriter -> sun.nio.cs.StreamEncoder -> Charset} pipeline. Both
 * {@code ByteArrayOutputStream} and the {@code Sink} subclass are non-allowlisted
 * real bytecode. The success marker is stored in a static field so the tail cannot
 * itself be a blocker that obscures the writer-graph walls.
 *
 * On completion the exact bytes drained into the sink are exposed via the static
 * {@code BYTES} field, so the harness can assert the real UTF-8 encoding of
 * {@code "hello\n"} ({@code {104,101,108,108,111,10}}) end-to-end rather than only
 * the byte count.
 */
public class WriterGraphProbe {
    static int RESULT = -1;
    static byte[] BYTES = null;

    static final class Sink extends OutputStream {
        final ByteArrayOutputStream buf = new ByteArrayOutputStream();
        @Override public void write(int b) { buf.write(b); }
        @Override public void write(byte[] b, int off, int len) { buf.write(b, off, len); }
    }

    public static void main(String[] args) throws Exception {
        Sink s = new Sink();
        OutputStreamWriter w = new OutputStreamWriter(s, "UTF-8");
        w.write("hello\n");
        w.flush();
        // success markers if we ever get here (cannot fail):
        RESULT = s.buf.size();
        BYTES = s.buf.toByteArray();
    }
}
