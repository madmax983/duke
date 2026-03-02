/**
 * Phase 10 fixture — calls methods in Callee via cross-class invokestatic.
 */
public class CrossCall {
    /** Simple cross-class call: Callee.add(a, b). */
    public static int callAdd(int a, int b) {
        return Callee.add(a, b);
    }

    /** Cross-class with single arg: Callee.doubleVal(n). */
    public static int callDouble(int n) {
        return Callee.doubleVal(n);
    }

    /** Nested cross-class: Callee.add(Callee.doubleVal(n), n). */
    public static int chainCall(int n) {
        return Callee.add(Callee.doubleVal(n), n);
    }

    /** Cross-class call to negate, then add: Callee.add(n, Callee.negate(n)). */
    public static int addNegated(int n) {
        return Callee.add(n, Callee.negate(n));
    }
}
