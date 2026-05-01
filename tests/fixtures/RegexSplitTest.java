import java.util.regex.Pattern;

public class RegexSplitTest {
    private static boolean eq(String actual, String expected) {
        return actual.equals(expected);
    }

    public static int defaultSplit() {
        String[] parts = Pattern.compile("[,;]").split("a,b;c,,d");
        return parts.length == 5
            && eq(parts[0], "a")
            && eq(parts[1], "b")
            && eq(parts[2], "c")
            && eq(parts[3], "")
            && eq(parts[4], "d") ? 1 : 0;
    }

    public static int preserveTrailingEmpties() {
        String[] exact = Pattern.compile("[,;]").split("a,b;c,,d", -1);
        String[] trailing = Pattern.compile("[,;]").split("a,b;c,,d,;", -1);
        return exact.length == 5
            && trailing.length == 7
            && eq(trailing[5], "")
            && eq(trailing[6], "") ? 1 : 0;
    }

    public static int defaultStripsTrailingEmpties() {
        String[] parts = Pattern.compile("[,;]").split("a,b;c,,d,;");
        return parts.length == 5 && eq(parts[4], "d") ? 1 : 0;
    }

    public static int positiveLimit() {
        String[] parts = Pattern.compile(",").split("a,b,c", 2);
        return parts.length == 2 && eq(parts[0], "a") && eq(parts[1], "b,c") ? 1 : 0;
    }

    public static int runAll() {
        return defaultSplit()
            + preserveTrailingEmpties()
            + defaultStripsTrailingEmpties()
            + positiveLimit();
    }

    public static void main(String[] args) {
        System.out.println(runAll());
    }
}
