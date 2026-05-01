import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class RegexFlagsTest {
    public static int constants() {
        return Pattern.CASE_INSENSITIVE == 2
            && Pattern.COMMENTS == 4
            && Pattern.MULTILINE == 8
            && Pattern.DOTALL == 32
            && Pattern.UNICODE_CASE == 64 ? 1 : 0;
    }

    public static int caseInsensitive() {
        boolean plain = Pattern.compile("duke").matcher("DUKE").find();
        boolean flagged = Pattern.compile("duke", Pattern.CASE_INSENSITIVE).matcher("DUKE").find();
        return !plain && flagged ? 1 : 0;
    }

    public static int multiline() {
        String input = "foo\nbar";
        boolean plain = Pattern.compile("^bar").matcher(input).find();
        boolean flagged = Pattern.compile("^bar", Pattern.MULTILINE).matcher(input).find();
        return !plain && flagged ? 1 : 0;
    }

    public static int dotall() {
        String input = "a\nb";
        boolean plain = Pattern.compile("a.b").matcher(input).matches();
        boolean flagged = Pattern.compile("a.b", Pattern.DOTALL).matcher(input).matches();
        return !plain && flagged ? 1 : 0;
    }

    public static int comments() {
        String regex = "a b # ignored\n c";
        boolean plain = Pattern.compile(regex).matcher("abc").matches();
        boolean flagged = Pattern.compile(regex, Pattern.COMMENTS).matcher("abc").matches();
        return !plain && flagged ? 1 : 0;
    }

    public static int unicodeCase() {
        String lowerUmlaut = "\u00FC";
        String upperUmlaut = "\u00DC";
        Matcher asciiOnly = Pattern.compile(lowerUmlaut, Pattern.CASE_INSENSITIVE).matcher(upperUmlaut);
        Matcher unicode = Pattern.compile(
            lowerUmlaut,
            Pattern.CASE_INSENSITIVE | Pattern.UNICODE_CASE
        ).matcher(upperUmlaut);
        return !asciiOnly.find() && unicode.find() ? 1 : 0;
    }

    public static int runAll() {
        return constants()
            + caseInsensitive()
            + multiline()
            + dotall()
            + comments()
            + unicodeCase();
    }

    public static void main(String[] args) {
        System.out.println(runAll());
    }
}
