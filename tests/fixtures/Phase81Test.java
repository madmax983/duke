import java.util.*;
import java.util.stream.*;

public class Phase81Test {

    // Custom exception class
    static class MyException extends RuntimeException {
        private final int code;
        MyException(String msg, int code) {
            super(msg);
            this.code = code;
        }
        int getCode() { return code; }
    }

    public static int testCustomException() {
        try {
            throw new MyException("oops", 42);
        } catch (MyException e) {
            return e.getCode(); // 42
        }
    }

    // Custom exception hierarchy
    static class AppException extends Exception {
        AppException(String msg) { super(msg); }
    }
    static class ValidationException extends AppException {
        ValidationException(String msg) { super(msg); }
    }

    public static int testExceptionHierarchy() {
        try {
            throw new ValidationException("bad input");
        } catch (AppException e) {
            return e.getMessage().length(); // "bad input".length() = 9
        }
    }

    // Exception chaining (cause)
    public static int testExceptionCause() {
        try {
            try {
                throw new IllegalArgumentException("root");
            } catch (IllegalArgumentException e) {
                throw new RuntimeException("wrapper", e);
            }
        } catch (RuntimeException e) {
            return e.getCause() != null ? 1 : 0; // 1
        }
    }

    // Exception message
    public static int testExceptionMessage() {
        try {
            throw new RuntimeException("hello world");
        } catch (RuntimeException e) {
            return e.getMessage().length(); // 11
        }
    }

    // ArithmeticException (divide by zero)
    public static int testDivisionByZero() {
        try {
            int x = 10 / 0;
            return x;
        } catch (ArithmeticException e) {
            return 7; // caught
        }
    }

    // UnsupportedOperationException
    public static int testUnsupportedOperation() {
        try {
            List<Integer> immutable = Collections.unmodifiableList(new ArrayList<>());
            immutable.add(1); // should throw
            return 0;
        } catch (UnsupportedOperationException e) {
            return 1; // caught
        }
    }

    // IllegalArgumentException
    public static int testIllegalArgument() {
        try {
            throw new IllegalArgumentException("bad arg");
        } catch (IllegalArgumentException e) {
            return e.getMessage().length(); // 7
        }
    }

    // IllegalStateException
    public static int testIllegalState() {
        try {
            throw new IllegalStateException("bad state");
        } catch (IllegalStateException e) {
            return e.getMessage().length(); // 9
        }
    }
}
