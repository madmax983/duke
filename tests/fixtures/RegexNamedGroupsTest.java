import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class RegexNamedGroupsTest {
    private static Matcher dateMatcher() {
        Pattern p = Pattern.compile("(?<year>\\d{4})-(?<month>\\d{2})-(?<day>\\d{2})");
        Matcher m = p.matcher("2026-04-30");
        m.find();
        return m;
    }

    public static int namedValues() {
        Matcher m = dateMatcher();
        return m.group("year").equals("2026")
            && m.group("month").equals("04")
            && m.group("day").equals("30") ? 1 : 0;
    }

    public static int namedStarts() {
        Matcher m = dateMatcher();
        return m.start("year") == 0 && m.start("month") == 5 && m.start("day") == 8 ? 1 : 0;
    }

    public static int namedEnds() {
        Matcher m = dateMatcher();
        return m.end("year") == 4 && m.end("month") == 7 && m.end("day") == 10 ? 1 : 0;
    }

    public static int unknownNameThrows() {
        try {
            dateMatcher().group("hour");
            return 0;
        } catch (IllegalArgumentException expected) {
            return 1;
        }
    }

    public static int runAll() {
        return namedValues() + namedStarts() + namedEnds() + unknownNameThrows();
    }

    public static void main(String[] args) {
        System.out.println(runAll());
    }
}
