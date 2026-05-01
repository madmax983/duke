import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class RegexResetTest {
    public static int resetWithNewInput() {
        Pattern p = Pattern.compile("\\d+");
        Matcher m = p.matcher("one 1");
        if (!m.find() || !m.group().equals("1")) return 0;
        if (m.reset("two 22") != m) return 0;
        try {
            m.start();
            return 0;
        } catch (IllegalStateException expected) {
        }
        return m.find() && m.start() == 4 && m.end() == 6 && m.group().equals("22") ? 1 : 0;
    }

    public static int resetSameInput() {
        Matcher m = Pattern.compile("\\d").matcher("1 2");
        if (!m.find() || !m.group().equals("1")) return 0;
        if (!m.find() || !m.group().equals("2")) return 0;
        if (m.reset() != m) return 0;
        return m.find() && m.group().equals("1") ? 1 : 0;
    }

    public static int runAll() {
        return resetWithNewInput() + resetSameInput();
    }

    public static void main(String[] args) {
        System.out.println(runAll());
    }
}
