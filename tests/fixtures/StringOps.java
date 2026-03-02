public class StringOps {
    /** Returns the length of "Hello". */
    public static int stringLength() {
        String s = "Hello";
        return s.length();
    }

    /** Returns 1 if two equal strings match. */
    public static int stringEquals() {
        String a = "Duke";
        String b = "Duke";
        return a.equals(b) ? 1 : 0;
    }

    /** Returns 0 if two different strings don't match. */
    public static int stringNotEquals() {
        String a = "Duke";
        String b = "Java";
        return a.equals(b) ? 1 : 0;
    }

    /** Returns the char at index 1 of "Hello" as int. */
    public static int charAtOne() {
        String s = "Hello";
        return (int) s.charAt(1);
    }
}
