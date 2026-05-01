import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class RegexGroupCountTest {
    public static int zeroGroups() {
        Matcher m = Pattern.compile("\\d+").matcher("abc123");
        if (!m.find() || m.groupCount() != 0) return 0;
        return m.group(0).equals("123") ? 1 : 0;
    }

    public static int twoGroups() {
        Matcher m = Pattern.compile("(\\w+)-(\\d+)").matcher("abc-123");
        if (!m.find() || m.groupCount() != 2) return 0;
        String combined = "";
        for (int i = 0; i <= m.groupCount(); i++) {
            combined = combined + "[" + m.group(i) + "]";
        }
        return combined.equals("[abc-123][abc][123]") ? 1 : 0;
    }

    public static int fourGroups() {
        Matcher m = Pattern.compile("(a)(b)(c)(d)").matcher("abcd");
        if (!m.find() || m.groupCount() != 4) return 0;
        String combined = "";
        for (int i = 0; i <= m.groupCount(); i++) {
            combined = combined + m.group(i);
        }
        return combined.equals("abcdabcd") ? 1 : 0;
    }

    public static int runAll() {
        return zeroGroups() + twoGroups() + fourGroups();
    }

    public static void main(String[] args) {
        System.out.println(runAll());
    }
}
