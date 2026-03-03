public class MoreNatives {
    /** Returns hashCode of a new Object (should be non-zero). */
    public static int objectHashCode() {
        Object o = new MoreNatives();
        return (o.hashCode() != 0) ? 1 : 0; // 1
    }

    /** Returns String.valueOf(42). Tests static native method. */
    public static int valueOfInt() {
        String s = String.valueOf(42);
        return s.length(); // 2 ("42" has 2 chars)
    }

    /** Prints without newline then with newline. */
    public static int printNoNewline() {
        System.out.print("AB");
        System.out.println("CD");
        return 1;
    }
}
