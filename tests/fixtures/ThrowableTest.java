public class ThrowableTest {
    static int testGetMessage() {
        Exception e = new Exception("hello");
        return e.getMessage().length();  // 5
    }

    static int testGetMessageNull() {
        Exception e = new Exception();
        return e.getMessage() == null ? 1 : 0;  // 1
    }

    static int testToString() {
        Exception e = new Exception("oops");
        String s = e.toString();
        return s.contains("oops") ? 1 : 0;  // 1
    }

    static int testCatchGetMessage() {
        try {
            throw new RuntimeException("fail");
        } catch (RuntimeException e) {
            return e.getMessage().length();  // 4
        }
    }
}
