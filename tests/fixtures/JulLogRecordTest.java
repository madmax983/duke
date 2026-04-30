import java.util.logging.Level;
import java.util.logging.LogRecord;

public final class JulLogRecordTest {
    public static int recordBasics() {
        LogRecord record = new LogRecord(Level.WARNING, "boom");
        if (record.getLevel() != Level.WARNING) return -1;
        if (!"boom".equals(record.getMessage())) return -2;
        record.setMessage("bang");
        if (!"bang".equals(record.getMessage())) return -3;

        RuntimeException thrown = new RuntimeException("x");
        record.setThrown(thrown);
        if (!"x".equals(record.getThrown().getMessage())) return -4;

        Object[] parameters = new Object[] { "alpha", Integer.valueOf(7) };
        record.setParameters(parameters);
        Object[] observed = record.getParameters();
        if (observed.length != 2) return -5;
        if (!"alpha".equals(observed[0])) return -6;
        if (((Integer) observed[1]).intValue() != 7) return -7;
        if (record.getMillis() <= 0L) return -8;
        return 1;
    }
}
