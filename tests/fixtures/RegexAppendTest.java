import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class RegexAppendTest {
    public static String replaceFish() {
        Pattern p = Pattern.compile("(\\w+) fish");
        Matcher m = p.matcher("one fish two fish red fish");
        StringBuilder sb = new StringBuilder();
        while (m.find()) {
            m.appendReplacement(sb, "$1 cats");
        }
        m.appendTail(sb);
        return sb.toString();
    }

    public static int runAll() {
        return replaceFish().equals("one cats two cats red cats") ? 1 : 0;
    }

    public static void main(String[] args) {
        System.out.println(replaceFish());
    }
}
