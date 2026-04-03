import java.util.Optional;

public class OptionalTest {
    static int testEmpty() {
        Optional opt = Optional.empty();
        return opt.isPresent() ? 1 : 0;  // 0
    }

    static int testOf() {
        Optional opt = Optional.of("hello");
        return opt.isPresent() ? 1 : 0;  // 1
    }

    static int testGet() {
        Optional opt = Optional.of("hello");
        return ((String) opt.get()).length();  // 5
    }

    static int testOrElse() {
        Optional opt = Optional.empty();
        String s = (String) opt.orElse("default");
        return s.length();  // 7
    }

    static int testOrElsePresent() {
        Optional opt = Optional.of("hi");
        String s = (String) opt.orElse("default");
        return s.length();  // 2
    }

    static int testIsEmpty() {
        Optional opt = Optional.empty();
        return opt.isEmpty() ? 1 : 0;  // 1
    }

    static int testOfNullable() {
        Optional opt = Optional.ofNullable(null);
        return opt.isPresent() ? 1 : 0;  // 0
    }
}
