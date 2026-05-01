import java.util.logging.Level;

public final class JulLevelStaticsTest {
    public static int staticsAndParse() {
        if (Level.SEVERE.intValue() != 1000) return -1;
        if (Level.INFO.intValue() != 800) return -2;
        if (Level.OFF.intValue() != Integer.MAX_VALUE) return -3;
        if (Level.ALL.intValue() != Integer.MIN_VALUE) return -4;
        if (Level.parse("INFO") != Level.INFO) return -5;
        if (Level.parse("800") != Level.INFO) return -6;
        if (!"INFO".equals(Level.INFO.getName())) return -7;
        if (!"INFO".equals(Level.INFO.toString())) return -8;
        return 1;
    }
}
