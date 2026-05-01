import java.util.regex.Pattern;
import java.util.regex.PatternSyntaxException;

public class RegexBackrefTest {
    public static int runAll() {
        try {
            Pattern duplicateWord = Pattern.compile("(\\w+)\\s+\\1");
            boolean yes = duplicateWord.matcher("hello hello").matches();
            boolean no = duplicateWord.matcher("hello world").matches();
            return yes && !no ? 1 : 0;
        } catch (PatternSyntaxException expected) {
            return expected.getMessage() == null ? 0 : 2;
        }
    }

    public static void main(String[] args) {
        System.out.println(runAll());
    }
}
