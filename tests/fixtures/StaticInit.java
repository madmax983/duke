public class StaticInit {
    static int x = 42;
    static int y = x + 8; // 50
    static int z;

    static {
        z = x + y; // 92
    }

    /** Returns the statically initialized value of x. */
    public static int getX() {
        return x; // 42
    }

    /** Returns the statically initialized value of y. */
    public static int getY() {
        return y; // 50
    }

    /** Returns the statically initialized value of z (from static block). */
    public static int getZ() {
        return z; // 92
    }

    /** Returns x + y + z. */
    public static int sum() {
        return x + y + z; // 184
    }
}
