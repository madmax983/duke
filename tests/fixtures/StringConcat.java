public class StringConcat {
    public static int concatLength() {
        String a = "Hello";
        String b = "World";
        String c = a.concat(b);
        return c.length();
    }

    public static int boolToString() {
        String s = String.valueOf(true);
        return s.length();
    }

    public static int longToString() {
        String s = String.valueOf(100L);
        return s.length();
    }

    public static int charToString() {
        String s = String.valueOf('A');
        return s.length();
    }

    public static int doubleToString() {
        String s = String.valueOf(3.14);
        return (s.length() > 0) ? 1 : 0;
    }
}
