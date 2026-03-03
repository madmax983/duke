public class LambdaTest {
    interface IntOp {
        int apply(int x);
    }

    public static int applyOp(IntOp op, int val) {
        return op.apply(val);
    }

    public static int testDouble() {
        return applyOp(x -> x * 2, 5);
    }

    public static int testCapture() {
        int base = 100;
        return applyOp(x -> x + base, 7);
    }

    public static int negate(int x) {
        return -x;
    }

    public static int testMethodRef() {
        return applyOp(LambdaTest::negate, 42);
    }

    public static int testMultiCapture() {
        int a = 10, b = 20;
        return applyOp(x -> x + a + b, 3);
    }
}
