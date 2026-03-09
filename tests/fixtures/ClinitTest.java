public class ClinitTest {
    static int VALUE;

    static {
        VALUE = 42;
    }

    public static int getValue() {
        return VALUE;
    }
}
