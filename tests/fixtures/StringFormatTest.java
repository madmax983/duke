public class StringFormatTest {
    static int testFormatString() {
        String s = String.format("hello %s", "world");
        return s.length();  // "hello world" = 11
    }

    static int testFormatInt() {
        String s = String.format("%d", 42);
        return s.length();  // "42" = 2
    }

    static int testFormatMultiple() {
        String s = String.format("%s=%d", "x", 7);
        return s.length();  // "x=7" = 3
    }

    static int testFormatDouble() {
        String s = String.format("%.2f", 3.14159);
        return s.length();  // "3.14" = 4
    }

    static int testFormatHex() {
        String s = String.format("%x", 255);
        return s.length();  // "ff" = 2
    }

    static int testFormatPercent() {
        String s = String.format("100%%");
        return s.length();  // "100%" = 4
    }

    static int testFormatNull() {
        String s = String.format("%s", (Object) null);
        return s.length();  // "null" = 4
    }

    static int testFormatSum() {
        int a = 10, b = 32;
        String s = String.format("%d+%d=%d", a, b, a + b);
        return s.length();  // "10+32=42" = 8
    }
}
