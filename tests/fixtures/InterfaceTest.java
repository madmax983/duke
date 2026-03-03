/** Tests invokeinterface dispatch. */
public class InterfaceTest {
    interface Adder {
        int add(int a, int b);
    }

    static class SimpleAdder implements Adder {
        public int add(int a, int b) {
            return a + b;
        }
    }

    static class DoubleAdder implements Adder {
        public int add(int a, int b) {
            return (a + b) * 2;
        }
    }

    /** Calls interface method on SimpleAdder. */
    public static int callSimple() {
        Adder a = new SimpleAdder();
        return a.add(3, 4); // 7
    }

    /** Calls interface method on DoubleAdder. */
    public static int callDouble() {
        Adder a = new DoubleAdder();
        return a.add(3, 4); // 14
    }

    /** Polymorphic dispatch: same interface, different impl. */
    public static int polymorphic(int which) {
        Adder a;
        if (which == 0) {
            a = new SimpleAdder();
        } else {
            a = new DoubleAdder();
        }
        return a.add(5, 3); // 8 or 16
    }
}
