import java.util.Objects;

public class ObjectsTest {
    static int testIsNull() {
        return Objects.isNull(null) ? 1 : 0;  // 1
    }

    static int testIsNullFalse() {
        return Objects.isNull("hello") ? 1 : 0;  // 0
    }

    static int testNonNull() {
        return Objects.nonNull("hello") ? 1 : 0;  // 1
    }

    static int testNonNullFalse() {
        return Objects.nonNull(null) ? 1 : 0;  // 0
    }

    static int testRequireNonNull() {
        String s = (String) Objects.requireNonNull("ok");
        return s.length();  // 2
    }

    static int testEquals() {
        return Objects.equals("abc", "abc") ? 1 : 0;  // 1
    }

    static int testEqualsBothNull() {
        return Objects.equals(null, null) ? 1 : 0;  // 1
    }

    static int testEqualsOneNull() {
        return Objects.equals(null, "x") ? 1 : 0;  // 0
    }

    static int testHashCode() {
        return Objects.hashCode(null);  // 0
    }
}
