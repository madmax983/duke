public class StringMethods {
    public static int testSubstring() {
        String s = "HelloWorld";
        String sub = s.substring(5);
        return sub.length(); // 5 ("World")
    }

    public static int testSubstringRange() {
        String s = "HelloWorld";
        String sub = s.substring(0, 5);
        return sub.length(); // 5 ("Hello")
    }

    public static int testIndexOf() {
        String s = "HelloWorld";
        return s.indexOf("World"); // 5
    }

    public static int testIndexOfNotFound() {
        String s = "HelloWorld";
        return s.indexOf("xyz"); // -1
    }

    public static int testContains() {
        String s = "HelloWorld";
        return s.contains("World") ? 1 : 0; // 1
    }

    public static int testIsEmpty() {
        String s = "";
        String t = "hi";
        return (s.isEmpty() && !t.isEmpty()) ? 1 : 0; // 1
    }

    public static int testCompareTo() {
        String a = "apple";
        String b = "banana";
        return (a.compareTo(b) < 0) ? 1 : 0; // 1 (a < b)
    }

    public static int testStartsWith() {
        String s = "HelloWorld";
        return s.startsWith("Hello") ? 1 : 0; // 1
    }

    public static int testEndsWith() {
        String s = "HelloWorld";
        return s.endsWith("World") ? 1 : 0; // 1
    }

    public static int testTrim() {
        String s = "  hi  ";
        return s.trim().length(); // 2
    }

    public static int testToCharArray() {
        String s = "AB";
        char[] chars = s.toCharArray();
        return chars[0] + chars[1]; // 65 + 66 = 131
    }
}
