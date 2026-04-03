public class IntegerStringConvTest {
    static int testToBinaryString() {
        return Integer.toBinaryString(10).length();  // "1010" = 4
    }

    static int testToHexString() {
        return Integer.toHexString(255).length();  // "ff" = 2
    }

    static int testToOctalString() {
        return Integer.toOctalString(8).length();  // "10" = 2
    }

    static int testToBinaryStringOne() {
        String s = Integer.toBinaryString(1);
        return s.equals("1") ? 1 : 0;  // 1
    }
}
