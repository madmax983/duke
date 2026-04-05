import java.util.*;
import java.util.stream.*;
import java.util.regex.*;

public class Phase77Test {

    // Pattern.matches (static)
    public static int testPatternMatches() {
        int r = 0;
        if (Pattern.matches("[0-9]+", "12345")) r += 1;
        if (!Pattern.matches("[0-9]+", "abc")) r += 2;
        return r; // 3
    }

    // String.matches (delegates to Pattern)
    public static int testStringMatches() {
        int r = 0;
        if ("hello123".matches(".*[0-9]+")) r += 1;
        if (!"hello".matches("[0-9]+")) r += 2;
        return r; // 3
    }

    // Pattern.compile + matcher find count
    public static int testPatternMatcher() {
        Pattern p = Pattern.compile("[a-z]+");
        Matcher m = p.matcher("hello world foo");
        int count = 0;
        while (m.find()) count++;
        return count; // 3
    }

    // String.replaceAll (regex)
    public static int testStringReplaceAll() {
        String s = "hello world 123";
        String result = s.replaceAll("[0-9]+", "NUM");
        return result.length(); // "hello world NUM" = 15
    }

    // String.replaceFirst
    public static int testStringReplaceFirst() {
        String s = "aaa bbb aaa";
        String result = s.replaceFirst("aaa", "xxx");
        return result.charAt(0) == 'x' ? result.length() : -1; // 11
    }

    // String.split with regex
    public static int testStringSplitRegex() {
        String s = "one::two::three";
        String[] parts = s.split("::");
        return parts.length; // 3
    }

    // Matcher.group
    public static int testMatcherGroup() {
        Pattern p = Pattern.compile("([0-9]+)");
        Matcher m = p.matcher("val=42,count=8");
        int sum = 0;
        while (m.find()) {
            sum += Integer.parseInt(m.group(1));
        }
        return sum; // 42 + 8 = 50
    }

    // String.format with width padding
    public static int testStringFormatPadding() {
        String s = String.format("%5d", 42); // "   42"
        return s.length(); // 5
    }
}
