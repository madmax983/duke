public class StringBuilderTest {
    static int testBasicAppend() {
        StringBuilder sb = new StringBuilder();
        sb.append("hello");
        String s = sb.toString();
        return s.length();  // expect 5
    }

    static int testChaining() {
        String s = new StringBuilder().append("a").append("bc").append("def").toString();
        return s.length();  // expect 6
    }

    static int testAppendInt() {
        String s = new StringBuilder().append("val=").append(42).toString();
        return s.length();  // "val=42" = 6
    }

    static int testAppendLong() {
        String s = new StringBuilder().append(100L).toString();
        return s.length();  // "100" = 3
    }

    static int testAppendBoolean() {
        String s = new StringBuilder().append(true).toString();
        return s.length();  // "true" = 4
    }

    static int testAppendChar() {
        String s = new StringBuilder().append('X').toString();
        return s.length();  // "X" = 1
    }

    static int testAppendDouble() {
        String s = new StringBuilder().append(3.14).toString();
        return s.length();  // "3.14" = 4
    }

    static int testAppendFloat() {
        String s = new StringBuilder().append(1.5f).toString();
        return s.length();  // "1.5" = 3
    }

    static int testInitWithString() {
        StringBuilder sb = new StringBuilder("start");
        sb.append("end");
        return sb.toString().length();  // "startend" = 8
    }

    static int testLength() {
        StringBuilder sb = new StringBuilder();
        sb.append("abc");
        return sb.length();  // expect 3
    }

    static int testLoopBuild() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 5; i++) {
            sb.append(i);
        }
        return sb.toString().length();  // "01234" = 5
    }

    static int testAppendString() {
        StringBuilder sb = new StringBuilder();
        String s = "hello";
        sb.append(s);
        return sb.toString().length();  // 5
    }
}
